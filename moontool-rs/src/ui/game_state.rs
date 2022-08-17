use itertools::{zip_eq, Itertools};
use lazy_static::lazy_static;
use tui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, List, ListItem, Row, Table, Widget},
};

use crate::{
    make_spans,
    radlands::{
        choices::Choice,
        locations::{CardRowIndex, ColumnIndex, Player},
        people::get_person_types,
        styles::*,
        Action, GameState,
    },
    ui::layout::Layout,
};

pub struct GameStateWidget<'a, 'ctype, 'str> {
    pub block: Block<'str>,
    pub game_state: &'a GameState<'ctype>,
    pub choice: Option<&'a Choice<'ctype>>,
}

impl GameStateWidget<'_, '_, '_> {
    fn render_player(&self, area: Rect, buf: &mut Buffer, player: Player) {
        // get the player's title line
        let n = player.number();
        let is_cur_player = player == self.game_state.cur_player;
        let title = if is_cur_player {
            // current player
            make_spans!(
                format!(" Player {n} ("),
                Span::styled(
                    format!("{} water", self.game_state.cur_player_water),
                    *WATER
                ),
                ") ",
            )
        } else {
            // other player
            Spans::from(format!(" Player {n} "))
        };

        // draw the title + border
        let mut block = Block::default().title(title);
        if is_cur_player {
            block = block
                .borders(Borders::ALL)
                .border_type(BorderType::Thick)
                .border_style(
                    Style::default()
                        .fg(Color::LightYellow)
                        .add_modifier(Modifier::BOLD),
                );
        } else {
            block = block
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray));
        }
        let inner_area = block.inner(area);
        block.render(area, buf);

        lazy_static! {
            // TODO: include event cards too
            static ref MAX_CARD_NAME_LEN: u16 = get_person_types().iter()
                .map(|person_type| person_type.name.len())
                .max().unwrap()
                .try_into().unwrap();

            static ref MAX_EVENT_NAME_LEN: u16 = get_person_types().iter()
                .map(|person_type| person_type.name.len())
                .max().unwrap()
                .try_into().unwrap();
        }

        let [hand_rect, events_rect, board_rect] = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints([
                Constraint::Length(*MAX_CARD_NAME_LEN + 5 + 1),
                Constraint::Length(*MAX_EVENT_NAME_LEN + 4 + 1),
                Constraint::Min(30),
            ])
            .split(inner_area);

        // TODO: make these widgets that can compute their width to make the layout smarter?
        self.render_hand(hand_rect, buf, player);
        self.render_events(events_rect, buf, player);
        self.render_board(board_rect, buf, player);
    }

    fn render_hand(&self, area: Rect, buf: &mut Buffer, player: Player) {
        let player_state = self.game_state.player(player);
        let mut items = player_state
            .hand
            .iter()
            .map(|(card_type, count)| {
                make_spans!(
                    card_type.styled_name(),
                    if count > 1 { format!(" (x{count})") } else { "".to_string() }
                )
            })
            .collect_vec();
        if player_state.has_water_silo {
            items.push(Span::styled("Water Silo", *WATER).into());
        } else if player_state.hand.is_empty() {
            items.push(Span::styled("<none>", *EMPTY).into());
        }

        List::new(items.into_iter().map(ListItem::new).collect_vec())
            .block(Block::default().title("Hand"))
            .render(area, buf);
    }

    fn render_events(&self, area: Rect, buf: &mut Buffer, player: Player) {
        let player_state = self.game_state.player(player);
        let items = player_state.events.iter().enumerate().map(|(i, event)| {
            make_spans!(
                format!("[{}] ", i + 1),
                match event {
                    Some(event) => event.styled_name(),
                    None => Span::styled("<none>", *EMPTY),
                }
            )
        });

        List::new(items.into_iter().map(ListItem::new).collect_vec())
            .block(Block::default().title("Events"))
            .render(area, buf);
    }

    fn render_board(&self, area: Rect, buf: &mut Buffer, player: Player) {
        // get the columns
        let table_columns = self.game_state.player(player).columns.iter().map(|col| {
            [
                col.person_slots[1].styled_name(),
                col.person_slots[0].styled_name(),
                col.camp.styled_name(),
            ]
            .into_iter()
            .map(Spans::from)
            .collect_vec()
        });
        let mut table_columns = table_columns.collect_vec();

        let min_column_widths = table_columns
            .iter()
            .map(|column| column.iter().map(|s| s.width()).max().unwrap() + 4)
            .collect_vec();

        // tag board items with associated option numbers based on the type of Choice
        let mut tag_location = |row: CardRowIndex, col: ColumnIndex, i: usize| {
            let tag = Span::from(format!("({}) ", i + 1));
            let cell = &mut table_columns[col.as_usize()][2 - row.as_usize()];
            cell.0.insert(0, tag);
        };
        match self.choice {
            Some(Choice::Action(choice)) if player == self.game_state.cur_player => {
                for (i, action) in choice.actions().iter().enumerate().rev() {
                    match action {
                        Action::UsePersonAbility(_ability, loc) => {
                            tag_location(loc.row().into(), loc.column(), i);
                        }
                        Action::UseCampAbility(_ability, col_index) => {
                            tag_location(CardRowIndex::camp(), *col_index, i);
                        }
                        _ => {}
                    }
                }
            }
            Some(Choice::PlayLoc(choice)) if player == choice.chooser() => {
                for (i, loc) in choice.locations().iter().enumerate().rev() {
                    // TODO: shift and make gaps
                    tag_location(loc.row().into(), loc.column(), i);
                }
            }
            Some(Choice::Damage(choice)) => {
                for (i, loc) in choice.locations().iter().enumerate().rev() {
                    if player == loc.player() {
                        tag_location(loc.row(), loc.column(), i);
                    }
                }
            }
            Some(Choice::Restore(choice)) if player == choice.chooser() => {
                for (i, loc) in choice.locations().iter().enumerate().rev() {
                    tag_location(loc.row(), loc.column(), i);
                }
            }
            Some(Choice::RescuePerson(choice)) if player == choice.chooser() => {
                let locations = self.game_state.player(choice.chooser()).person_locs();
                for (i, loc) in locations.collect_vec().into_iter().enumerate().rev() {
                    tag_location(loc.row().into(), loc.column(), i);
                }
            }
            Some(Choice::DamageColumn(choice)) if player == choice.chooser().other() => {
                for (i, col) in choice.columns().iter().enumerate().rev() {
                    if !choice.people_only() {
                        tag_location(CardRowIndex::camp(), *col, i);
                    }
                    for (row, _) in self
                        .game_state
                        .player(player)
                        .column(*col)
                        .enumerate_people()
                    {
                        tag_location(row.into(), *col, i);
                    }
                }
            }
            _ => {}
        }

        // center the cells in their columns
        let column_widths = zip_eq(&table_columns, min_column_widths)
            .map(|(column, min_width)| {
                let actual_width = column.iter().map(|s| s.width()).max().unwrap();
                actual_width.max(min_width)
            })
            .collect_vec();
        for (column, &col_width) in zip_eq(&mut table_columns, &column_widths) {
            for spans in column {
                let cell_width = spans.width();
                if cell_width < col_width {
                    // pad the cell to center it in the column
                    let left_padding = (col_width - cell_width) / 2;
                    spans.0.insert(0, Span::raw(" ".repeat(left_padding)));
                }
            }
        }

        // transpose it into a list of rows
        let mut table_columns = table_columns
            .into_iter()
            .map(|col| col.into_iter())
            .collect_vec();
        let table_rows =
            (0..3).map(|_| Row::new(table_columns.iter_mut().map(|col| col.next().unwrap())));

        // build and render the final table
        Table::new(table_rows)
            .block(Block::default().title("Board"))
            .widths(
                &column_widths
                    .into_iter()
                    .map(|w| Constraint::Length(w.try_into().unwrap()))
                    .collect_vec(),
            )
            .column_spacing(2)
            .render(area, buf);
    }
}

impl Widget for GameStateWidget<'_, '_, '_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // first, render the block
        let inner_area = self.block.inner(area);
        self.block.clone().render(area, buf);

        if inner_area.area() == 0 {
            return;
        }

        // render the game state
        let player_rects = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(inner_area);
        for (i, player_rect) in player_rects.into_iter().enumerate() {
            let player = match i {
                0 => Player::Player1,
                1 => Player::Player2,
                _ => unreachable!(),
            };

            self.render_player(player_rect, buf, player);
        }

        // TODO: show the number of cards in the deck/discard?
    }
}
