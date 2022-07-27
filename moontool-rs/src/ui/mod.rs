mod game_state;
mod game_thread;
mod layout;

use std::{
    collections::VecDeque,
    io, mem, panic,
    sync::{
        mpsc::{self, SyncSender},
        Arc, Mutex,
    },
    thread,
    time::Instant,
};

use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use itertools::Itertools;
use lazy_static::lazy_static;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Corner, Direction},
    style::{Color, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;

use crate::radlands::{
    camps::{get_camp_types, CampType},
    choices::Choice,
    locations::Player,
    people::{get_person_types, PersonType},
    GameResult, GameState,
};

use self::{game_state::GameStateWidget, layout::Layout};

/// Spawns a new thread along with a monitor thread that will send a RedrawEvent::Abort
/// if the main spawned thread panics.
fn spawn_monitored_thread<T: Send + 'static>(
    name: &str,
    event_tx: mpsc::Sender<RedrawEvent>,
    f: impl FnOnce() -> T + Send + 'static,
) -> io::Result<thread::JoinHandle<T>> {
    // spawn the main work thread
    let builder = thread::Builder::new().name(name.into());
    let join_handle = builder.spawn(f)?;

    // spawn the monitoring thread
    let builder = thread::Builder::new().name(format!("panic monitor: {name}"));
    let join_handle2 = builder.spawn(move || {
        match join_handle.join() {
            Ok(value) => value, // forward the value
            Err(_) => {
                // the worker thread panicked
                event_tx
                    .send(RedrawEvent::Abort)
                    .expect("Failed to send Abort event");
                panic!("Monitored thread panicked");
            }
        }
    })?;

    Ok(join_handle2)
}

lazy_static! {
    static ref USER_INPUT_REQUESTS: Arc<Mutex<VecDeque<SyncSender<String>>>> =
        Arc::new(Mutex::new(VecDeque::new()));
}

// Gets a String of user input from the UI thread. Blocks until the user submits.
pub fn get_user_input() -> String {
    let (tx, rx) = mpsc::sync_channel(1);
    USER_INPUT_REQUESTS.lock().unwrap().push_back(tx);
    rx.recv().expect("Failed to recv() user input")
}

struct HistoryEntry<'ctype> {
    game_state: GameState<'ctype>,
    choice: Choice<'ctype>,
    chosen_option: usize,
}

impl<'ctype> HistoryEntry<'ctype> {
    fn format(&mut self) -> Spans<'ctype> {
        // TODO: this function shouldn't require &mut self
        // The issue is with GameView - make GameViewMut?
        self.choice
            .format_option(self.chosen_option, &mut self.game_state)
    }
}

enum InputMode {
    Normal,
    Editing,
}

/// An event that triggers a redraw.
enum RedrawEvent {
    Input(Event),
    GameUpdate(GameState<'static>, Result<Choice<'static>, GameResult>),
    Abort,
}

struct AppState {
    frame_num: usize,

    /// Current value of the input box
    input: String,
    /// Current input mode
    input_mode: InputMode,

    game_history: Arc<Mutex<Vec<HistoryEntry<'static>>>>,
    log_messages: Vec<String>,
    options_height: u16,

    cur_state: GameState<'static>,
    cur_choice: Result<Choice<'static>, GameResult>,
}

impl AppState {
    fn run(&mut self) -> io::Result<()> {
        // create a channel for sending events to the UI to trigger redraws
        let (event_tx, event_rx) = mpsc::channel();

        // setup terminal
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, Hide)?;

        fn restore_terminal() -> io::Result<()> {
            disable_raw_mode()?;
            execute!(io::stdout(), LeaveAlternateScreen, Show)
        }

        // set a hook that restores the terminal in case of a panic
        let original_hook = std::panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            restore_terminal().expect("error restoring terminal in panic hook");
            original_hook(panic_info);
        }));

        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // spawn a thread to generate RedrawEvents for terminal input events
        let event_tx2 = event_tx.clone();
        spawn_monitored_thread("input event thread", event_tx.clone(), move || loop {
            let event = event::read().expect("Failed to read crossterm event");
            event_tx2
                .send(RedrawEvent::Input(event))
                .expect("Failed to send crossterm event");
        })?;

        'main_loop: loop {
            // update the app state
            self.frame_num += 1;

            // draw a UI frame
            let start = Instant::now();
            terminal.draw(|f| ui(f, self))?;
            self.log_messages
                .push(format!("Frame took {:?}", start.elapsed()));

            if self.frame_num == 1 {
                // launch the game thread after drawing the first frame
                // (this makes panic messages nicer if it immediately panics)
                let game_history = self.game_history.clone();
                let initial_state = self.cur_state.clone();
                let initial_choice = self.cur_choice.clone();
                let event_tx2 = event_tx.clone();
                spawn_monitored_thread("game thread", event_tx.clone(), move || {
                    game_thread::game_thread_main(
                        initial_state,
                        initial_choice,
                        event_tx2,
                        game_history,
                    )
                })?;
            }

            // wait for events and handle them
            let mut event = event_rx.recv().expect("event channel disconnected");
            loop {
                // handle the event
                match event {
                    RedrawEvent::Input(event) => {
                        if let Event::Key(key) = event {
                            if self.handle_key_event(key) {
                                break 'main_loop;
                            }
                        }
                    }
                    RedrawEvent::GameUpdate(new_state, new_choice) => {
                        self.cur_state = new_state;
                        self.cur_choice = new_choice;
                    }
                    RedrawEvent::Abort => break 'main_loop,
                }

                // get the next event (if any is available now)
                event = match event_rx.try_recv() {
                    Ok(event) => event,
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        panic!("event channel disconnected");
                    }
                }
            }
        }

        let _ = panic::take_hook();

        // restore terminal
        restore_terminal()?;

        Ok(())
    }

    /// Handles a KeyEvent. Returns true if the app should quit.
    fn handle_key_event(&mut self, key: KeyEvent) -> bool {
        match self.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Enter if !USER_INPUT_REQUESTS.lock().unwrap().is_empty() => {
                    self.input_mode = InputMode::Editing;
                }
                KeyCode::Char('s') => {
                    // shrink the options pane to fit
                    self.options_height = 0;
                }
                KeyCode::Char('q') => {
                    // quit the app
                    return true;
                }
                _ => {}
            },
            InputMode::Editing => match key.code {
                KeyCode::Enter if !self.input.is_empty() => {
                    let mut input_requests = USER_INPUT_REQUESTS.lock().unwrap();
                    if let Some(tx) = input_requests.pop_front() {
                        let input = mem::take(&mut self.input);
                        tx.send(input).expect("Failed to send user input");
                    }
                }
                KeyCode::Char(c) => {
                    self.input.push(c);
                }
                KeyCode::Backspace => {
                    self.input.pop();
                }
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                }
                _ => {}
            },
        }
        false // don't quit the app
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut AppState) {
    // compute the top-level layout rects
    let [left_rect, right_rect] = Layout::default()
        .direction(Direction::Horizontal)
        // .margin(1)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(f.size());

    let max_player_height = [Player::Player1, Player::Player2]
        .into_iter()
        .map(|player| {
            let player_state = app.cur_state.player(player);
            let hand_len =
                player_state.hand.count_unique() + (player_state.has_water_silo as usize);
            usize::max(hand_len, 4) + 5
        })
        .max()
        .unwrap();
    let game_state_height = max_player_height * 2 + 1;

    let [game_state_rect, stats_rect] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(game_state_height.try_into().unwrap()),
            Constraint::Min(5),
        ])
        .split(right_rect);

    // render the log pane
    let mut options = Vec::new();
    if let Ok(choice) = &app.cur_choice {
        let num_options = choice.num_options(&app.cur_state);
        options = (0..num_options)
            .map(|i| {
                let mut spans = choice.format_option(i, &mut app.cur_state);
                let num_string = format!("({})", i + 1);
                spans.0.insert(0, Span::raw(format!("{num_string:>5}  ")));
                ListItem::new(spans)
            })
            .rev()
            .collect();
    }

    let mut history_items = {
        let mut game_history = app.game_history.lock().unwrap();
        game_history
            .iter_mut()
            .rev()
            .map(|entry| {
                let mut spans = entry.format();
                let chooser = entry.choice.chooser(&entry.game_state);
                spans.0.insert(0, Span::raw(format!("{chooser:?}:  ")));
                ListItem::new(spans)
            })
            .collect_vec()
    };
    if let Err(game_result) = app.cur_choice {
        let message = match game_result {
            GameResult::P1Wins => "Player 1 wins!",
            GameResult::P2Wins => "Player 2 wins!",
            GameResult::Tie => "The game ends in a tie!",
        };
        history_items.insert(0, ListItem::new(message));
    }

    let desired_options_height: u16 = (options.len() + 1).try_into().unwrap();
    app.options_height = app.options_height.max(desired_options_height);

    let [log_rect, options_rect, input_rect] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(app.options_height),
            Constraint::Length(3),
        ])
        .split(left_rect);

    let block = Block::default()
        .title(" Log ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL - Borders::BOTTOM);
    let logs = List::new(history_items)
        .block(block)
        .start_corner(Corner::BottomLeft);
    f.render_widget(logs, log_rect);

    let block = Block::default()
        .title(" Options ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL - Borders::BOTTOM);
    let options = List::new(options)
        .block(block)
        .start_corner(Corner::BottomLeft);
    f.render_widget(options, options_rect);

    // render the input box
    let input = Paragraph::new(app.input.as_ref())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow),
        })
        .block(
            Block::default()
                .title(" Input ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL),
        );
    f.render_widget(input, input_rect);
    match app.input_mode {
        // in Normal mode, hide the cursor (Frame's default)
        InputMode::Normal => {}

        // in Editing mode, make the cursor visible and ask tui to put it at
        // the specified coordinates after rendering
        InputMode::Editing => f.set_cursor(
            // put cursor past the end of the input text
            input_rect.x + app.input.width() as u16 + 1,
            // move one line down, from the border to the input line
            input_rect.y + 1,
        ),
    }

    // render the game state pane
    let block = Block::default()
        .title(" Game State ")
        .title_alignment(Alignment::Center)
        .borders(Borders::NONE);
    f.render_widget(
        GameStateWidget {
            block,
            game_state: &app.cur_state,
            actions: if let Ok(Choice::Action(choice)) = &app.cur_choice {
                choice.actions()
            } else {
                &[]
            },
        },
        game_state_rect,
    );

    // render the stats pane
    let block = Block::default()
        .title(" Stats ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL);
    f.render_widget(block, stats_rect);
}

pub(crate) fn main() -> io::Result<()> {
    lazy_static! {
        static ref CAMP_TYPES: Vec<CampType> = get_camp_types();
        static ref PERSON_TYPES: Vec<PersonType> = get_person_types();
    }
    let (game_state, choice) = GameState::new(&CAMP_TYPES, &PERSON_TYPES);

    let mut app = AppState {
        frame_num: 0,
        input: String::new(),
        input_mode: InputMode::Normal,
        game_history: Arc::new(Mutex::new(Vec::new())),
        log_messages: Vec::new(),
        options_height: 0,
        cur_state: game_state,
        cur_choice: Ok(choice),
    };

    app.run()
}
