// cppimport
#include <pybind11/pybind11.h>
// #include <pybind11/stl.h>
#include <algorithm>
#include <random>
#include <sstream>
#include <iostream>
using std::cout;
using std::cerr;
using std::endl;

namespace py = pybind11;


std::random_device rand_dev;
std::default_random_engine rand_eng{rand_dev()};


/// Type used for probabilities.
using prob_t = double;

/// Computes the binomial coefficient (n choose k).
/// Adapted from https://en.wikipedia.org/wiki/Binomial_coefficient#In_programming_languages
uint64_t binom(uint64_t n, uint64_t k) {
    if (k > n) return 0;
    if (k == 0 || k == n) return 1;
    
    k = std::min(k, n - k);  // take advantage of symmetry
    uint64_t c = 1;
    for (uint64_t i = 0; i < k; i++) {
        c = c * (n - i) / (i + 1);
    }
    return c;
}


struct State;
class Solver;
class CardType {
public:
    CardType(char letter, const char* color):
            letter(letter), color(color), sort_order(++max_sort_order) {}
    CardType(char letter, const char* color, int sort_order):
            letter(letter), color(color), sort_order(sort_order) {
        max_sort_order = std::max(max_sort_order, sort_order);
    }
    virtual ~CardType() {}

    /// Returns the completion probability after playing this card
    /// when starting in the given state.
    virtual prob_t play(const State& state, Solver& solver) const = 0;

    char letter;
    const char* color;  // ANSI color escape code for console printing
    int sort_order;

private:
    static int max_sort_order;
};
int CardType::max_sort_order = 0;


/// A multiset of cards.
struct Cards {
    Cards() {}
    Cards(std::initializer_list<std::pair<const CardType*, size_t>> init):cards(init.begin(), init.end()) {}

    template<class Iter>
    Cards(Iter first, Iter last) {
        for (auto it = first; it != last; ++it) {
            add(*it);
        }
    }

    std::unordered_map<const CardType*, size_t> cards;

    /// Adds n of the given card type to this multiset.
    void add(const CardType* type, size_t n = 1) {
        if (n == 0) return;  // adding 0 cards is a no-op
        auto it = cards.find(type);
        if (it == cards.end()) {
            cards.emplace(type, n);
        } else {
            it->second += n;
        }
    }

    /// Removes 1 of the given card type from this multiset.
    void remove(const CardType* type) {
        auto it = cards.find(type);
        if (it == cards.end()) {
            throw std::runtime_error("Tried to remove a type of card that wasn't there");
        } else {
            if (--it->second == 0) {
                cards.erase(it);
            }
        }
    }

    /// Removes n of the given card type from this multiset.
    void remove(const CardType* type, size_t n) {
        if (n == 0) return;  // removing 0 cards is a no-op
        auto it = cards.find(type);
        if (it == cards.end()) {
            throw std::runtime_error("Tried to remove a type of card that wasn't there");
        } else {
            if (n > it->second) {
                throw std::runtime_error("Tried to remove more of a card than are present");
            } else if ((it->second -= n) == 0) {
                cards.erase(it);
            }
        }
    }

    /// Removes all cards of the given type from this multiset.
    void remove_all(const CardType* type) {
        auto it = cards.find(type);
        if (it == cards.end()) {
            throw std::runtime_error("Tried to remove a type of card that wasn't there");
        } else {
            cards.erase(it);
        }
    }

    /// Returns the number of cards in this multiset.
    size_t size() const {
        size_t total = 0;
        for (auto&& entry : cards) {
            total += entry.second;
        }
        return total;
    }

    /// Returns whether this multiset contains no cards / is the empty multiset.
    bool is_empty() const noexcept {
        return cards.empty();
    }

    /// Returns a string representation of this multiset.
    std::string to_string() const {
        std::string str;
        for (auto&& entry : cards) {
            for (size_t n = 0; n < entry.second; n++) {
                str += entry.first->letter;
            }
        }
        return str;
    }

    /// Returns a sorted & colorized string representation of this multiset,
    /// suitable to be printed to a terminal.
    std::string to_console_string() const {
        if (is_empty()) return "\033[90m<no cards>\033[0m";

        // put the card types into a vector and sort them
        std::vector<std::pair<const CardType*, size_t>> card_types{cards.begin(), cards.end()};
        std::sort(card_types.begin(), card_types.end(), [](auto&& a, auto&& b) {
            return a.first->sort_order < b.first->sort_order;
        });

        // build the string representation
        std::string str;
        for (auto&& entry : card_types) {
            str += "\033[";
            str += entry.first->color;
            str += 'm';
            for (size_t n = 0; n < entry.second; n++) {
                str += entry.first->letter;
            }
        }
        str += "\033[0m";
        return str;
    }

    Cards& operator+=(const Cards& other) {
        for (auto&& entry : other.cards) {
            auto it = cards.find(entry.first);
            if (it == cards.end()) {
                cards.emplace(entry);
            } else {
                it->second += entry.second;
            }
        }
        return *this;
    }

    Cards operator+(const Cards& other) const {
        Cards new_cards = *this;
        new_cards += other;
        return new_cards;
    }

    bool operator==(const Cards& other) const {
        return cards == other.cards;
    }

    /// Draws (up to) n random cards from this multiset.
    /// Returns the new set, and the drawn cards.
    std::pair<Cards, Cards> draw_random(size_t n) const {
        std::vector<const CardType*> card_list;
        for (auto&& entry : cards) {
            for (size_t n = 0; n < entry.second; n++) {
                card_list.push_back(entry.first);
            }
        }
        if (n >= card_list.size()) {
            return {Cards(), *this};
        }
        std::shuffle(card_list.begin(), card_list.end(), rand_eng);
        auto split_iter = card_list.begin() + n;
        return {
            Cards(split_iter, card_list.end()),
            Cards(card_list.begin(), split_iter),
        };
    }

    /// Enumerates the possible unique draws of n cards from this multiset,
    /// calling the provided function for each:
    ///     func(<reduced deck>, <drawn cards>, <probability>)
    /// The enumeration order is not defined, and may be different even for
    /// equivalent multisets.
    /// (Note: There may be room to optimize this function.)
    template<class Func>
    void for_each_draw(size_t n, Func&& func) const {
        /// Tracks the state for a card type.
        struct ct_state_t {
            const CardType* type;  // the type of card
            size_t num_in_deck;  // how many are available in the deck
            size_t n_remaining;  // number of remaining cards that must be drawn
            size_t num_drawn;  // number drawn of this type
        };

        const int num_card_types = cards.size();
        if (num_card_types == 0) {
            // (*this) is an empty multiset; only one possible draw (nothing)
            func(*this, *this, prob_t(1));
            return;
        }

        // create an array of state objects
        ct_state_t states[num_card_types];
        int i = 0;
        size_t total_cards = 0;
        for (auto&& entry : cards) {
            states[i].type = entry.first;
            total_cards += (states[i].num_in_deck = entry.second);
            i++;
        }
        if (total_cards > 62) {
            cerr << "! Cards::for_each_draw: Total # of cards is " << total_cards;
            cerr << ".\n! Above 62, uint64_t may overflow in binomial";
            cerr << " coefficient calculations." << endl;
        }

        if (n > total_cards) n = total_cards;  // will only draw up to total_cards
        uint64_t prob_denominator = binom(total_cards, n);
        prob_t prob_norm = 1 / prob_t(prob_denominator);

        i = 0;
        states[0].n_remaining = n;
        states[0].num_drawn = 0;
        while (i >= 0) {
            /*/DEBUG STUFF
            cout << endl;
            for (int j = 0; j <= i; j++) {
                cout << states[j].num_drawn << ' ';
            }
            cout << endl;
            for (int j = 0; j <= i; j++) {
                cout << states[j].num_in_deck << ' ';
            }
            cout << endl;//*/

            // check how many more cards must be drawn
            size_t remaining = states[i].n_remaining - states[i].num_drawn;
            if (remaining == 0) {
                // found a valid draw set; report it
                Cards reduced_deck, drawn;
                prob_t prob_numerator = 1;
                for (int j = 0; j <= i; j++) {
                    drawn.add(states[j].type, states[j].num_drawn);
                    reduced_deck.add(states[j].type, states[j].num_in_deck - states[j].num_drawn);

                    // note: these binomial coefficients could be computed incrementally
                    // a la dynamic programming, which may(?) be faster in many(?) cases
                    prob_numerator *= binom(states[j].num_in_deck, states[j].num_drawn);
                }
                for (int j = i+1; j < num_card_types; j++) {
                    reduced_deck.add(states[j].type, states[j].num_in_deck);
                }
                prob_t prob = prob_numerator * prob_norm;
                func(const_cast<const Cards&>(reduced_deck), const_cast<const Cards&>(drawn), prob);
            } else {
                if (++i == num_card_types) {
                    // went through all card types without drawing enough cards;
                    // don't recurse (just loop again)
                    i--;
                } else {
                    // recurse to try drawing more cards of different types
                    states[i].n_remaining = remaining;
                    states[i].num_drawn = 0;
                    continue;
                }
            }

            // draw another of this type of card (and loop again if not done)
            while (++states[i].num_drawn > std::min(states[i].n_remaining, states[i].num_in_deck)) {
                // tried every number of this type of card; "return" up a level
                i--;
            }
        }
    }
};

std::ostream& operator<<(std::ostream& os, const Cards& cards) {
    return os << cards.to_console_string();
}


/// Requirements or sub-requirements for a contract.
struct Requirements {
    size_t reactors  = 0;
    size_t thrusters = 0;
    size_t shields   = 0;
    size_t damage    = 0;
    size_t crew      = 0;

    #define DEF_SUB_REQ(req_name) \
        void sub_##req_name(size_t n) {\
            if (n >= req_name) req_name = 0;\
            else req_name -= n;\
        }
    DEF_SUB_REQ(reactors)
    DEF_SUB_REQ(thrusters)
    DEF_SUB_REQ(shields)
    DEF_SUB_REQ(damage)
    DEF_SUB_REQ(crew)
    #undef DEF_SUB_REQ

    bool is_empty() const {
        return reactors == 0 && thrusters == 0 && shields == 0 && damage == 0 && crew == 0;
    }

    /// Returns a (optionally colorized) string representation of this
    /// requirements set.
    std::string to_string(bool color = false) const;

    bool operator==(const Requirements& other) const {
        return (
            reactors == other.reactors &&
            thrusters == other.thrusters &&
            shields == other.shields &&
            damage == other.damage &&
            crew == other.crew
        );
    }
};


/// A description of the game state while completing the contract.
struct State {
    size_t actions;
    Cards hand;
    Cards draw_pile;
    Requirements requirements;

    bool operator==(const State& other) const {
        return (
            actions == other.actions &&
            hand == other.hand &&
            draw_pile == other.draw_pile &&
            requirements == other.requirements
        );
    }
};


// hashing implementations
template <class T>
inline void hash_combine(std::size_t& seed, const T& v) {
    // this function is basically stolen from boost::hash_combine
    using std::hash;
    hash<T> hasher;
    seed ^= hasher(v) + 0x9e3779b9 + (seed<<6) + (seed>>2);
}
namespace std {
    template <>
    struct hash<Cards> {
        size_t operator()(const Cards& c) const {
            // it's likely this is a terrible way to hash a set
            size_t seed = 0;
            for (auto&& entry : c.cards) {
                size_t seed2 = 0;
                hash_combine(seed2, entry.second);
                hash_combine(seed2, entry.first);
                seed ^= seed2;
            }
            return seed;
        }
    };
    template <>
    struct hash<Requirements> {
        size_t operator()(const Requirements& r) const {
            size_t seed = 0;
            hash_combine(seed, r.reactors);
            hash_combine(seed, r.thrusters);
            hash_combine(seed, r.shields);
            hash_combine(seed, r.damage);
            hash_combine(seed, r.crew);
            return seed;
        }
    };
    template <>
    struct hash<State> {
        size_t operator()(const State& s) const {
            size_t seed = 0;
            hash_combine(seed, s.actions);
            hash_combine(seed, s.hand);
            hash_combine(seed, s.draw_pile);
            hash_combine(seed, s.requirements);
            return seed;
        }
    };
}


class Solver {
public:
    prob_t get_completion_probability(const State& state) {
        explore_count++;

        // check base cases
        if (state.requirements.is_empty()) {
            return 1.0;  // goal state found (solve probability: 100%)
        }
        if (state.actions == 0) {
            return 0.0;  // out of actions (solve probability: 0%)
            // TODO: what if you're able to play a card without an action?
        }

        // check if the result has been memoized from a previous call
        auto it = explored_states.find(state);
        if (it != explored_states.end()) {
            // cout << "state memoized" << endl;
            return it->second;  // this state has already been explored
        }

        // recurse for the different cards available to play
        prob_t max_solve_prob = 0;  // this tracks the solve probability for the best possible action
        for (auto&& entry : state.hand.cards) {
            prob_t solve_prob = entry.first->play(state, *this);
            max_solve_prob = std::max(max_solve_prob, solve_prob);
            // if (max_solve_prob == 1) break;  // can't get better than 100%
        }

        explored_states.emplace(state, max_solve_prob);  // memoize the result
        return max_solve_prob;
    }

    size_t explored_states_count() const {
        return explore_count;
    }

private:
    std::unordered_map<State, prob_t> explored_states;
    size_t explore_count = 0;
};


// card type definitions

class ReactorCard: public CardType {
public:
    ReactorCard():CardType('R', "96") {}
    prob_t play(const State& state, Solver& solver) const override {
        State new_state = state;
        new_state.hand.remove(this);
        new_state.actions += 1;  // -1 action, then +2 actions
        new_state.requirements.sub_reactors(1);
        return solver.get_completion_probability(new_state);
    }
};
const ReactorCard REACTOR;

class ThrusterCard: public CardType {
public:
    ThrusterCard():CardType('T', "93") {}
    prob_t play(const State& state, Solver& solver) const override {
        State new_state;
        new_state.actions = state.actions - 1;
        new_state.requirements = state.requirements;
        new_state.requirements.sub_thrusters(1);

        Cards hand_before_draw = state.hand;
        hand_before_draw.remove(this);

        // sum over all possible draws of 2 cards
        prob_t total_prob = 0;
        state.draw_pile.for_each_draw(2, [&](const Cards& new_draw_pile, const Cards& drawn, prob_t prob) {
            // cout << "draw pile: " << new_draw_pile << ", drew: " << drawn << ", prob=" << prob << endl;
            new_state.draw_pile = std::move(new_draw_pile);
            new_state.hand = hand_before_draw;
            new_state.hand += drawn;
            total_prob += prob * solver.get_completion_probability(new_state);
        });
        return total_prob;
    }
};
const ThrusterCard THRUSTER;

class ShieldCard: public CardType {
public:
    ShieldCard():CardType('S', "92") {}
    prob_t play(const State& state, Solver& solver) const override {
        State new_state = state;
        new_state.hand.remove(this);
        new_state.actions -= 1;
        new_state.requirements.sub_shields(1);
        // TODO: reduce hazard too
        return solver.get_completion_probability(new_state);
    }
};
const ShieldCard SHIELD;

class DamageCard: public CardType {
public:
    DamageCard():CardType('D', "33") {}
    prob_t play(const State& state, Solver& solver) const override {
        State new_state = state;
        new_state.hand.remove(this);
        new_state.actions -= 1;
        new_state.requirements.sub_damage(1);
        return solver.get_completion_probability(new_state);
    }
};
const DamageCard DAMAGE;

class MissCard: public CardType {
public:
    MissCard():CardType('M', "37") {}
    prob_t play(const State& state, Solver& solver) const override {
        State new_state = state;
        new_state.hand.remove(this);
        new_state.actions -= 1;
        return solver.get_completion_probability(new_state);
    }
};
const MissCard MISS;


std::string Requirements::to_string(bool color) const {
    const struct {
        char letter;
        const char* color;
        size_t count;
    } reqs[] = {
        {REACTOR.letter, REACTOR.color, reactors},
        {THRUSTER.letter, THRUSTER.color, thrusters},
        {SHIELD.letter, SHIELD.color, shields},
        {DAMAGE.letter, DAMAGE.color, damage},
        {'C', "95", crew},
    };

    std::ostringstream buf;
    bool empty = true;
    for (auto&& req : reqs) {
        if (req.count > 0) {
            if (!empty) buf << ", ";
            if (color) buf << "\033[" << req.color << 'm';
            buf << req.letter;
            if (color) buf << "\033[0m\xc3\x97";
            else buf << 'x';
            buf << req.count;
            empty = false;
        }
    }

    return buf.str();
}


const Cards DEFAULT_DECK{
    {&REACTOR,  3},
    {&THRUSTER, 2},
    {&SHIELD,   2},
    {&DAMAGE,   2},
    {&MISS,     1},
};


PYBIND11_MODULE(contract_solver_stuff, m) {
    m.def("get_default_deck", []() { return DEFAULT_DECK; });

    py::class_<CardType>(m, "CardType")
        .def_readonly("letter", &CardType::letter);

    py::class_<Cards>(m, "Cards")
        .def(py::init<>())
        .def("add", &Cards::add, py::arg("type"), py::arg("n") = 1)
        .def("remove", static_cast<void (Cards::*)(const CardType*)>(&Cards::remove), py::arg("type"))
        .def("remove", static_cast<void (Cards::*)(const CardType*, size_t)>(&Cards::remove), py::arg("type"), py::arg("n"))
        .def("draw_random", &Cards::draw_random, py::arg("n"))
        .def("to_console_string", &Cards::to_console_string)
        .def("__len__", [](const Cards& cards) { return cards.size(); })
        .def("__str__", [](const Cards& cards) { return cards.to_string(); })
        .def("__repr__", [](const Cards& cards) { return "Cards<"+cards.to_string()+">"; });

    py::class_<Requirements>(m, "Requirements")
        .def(py::init<size_t, size_t, size_t, size_t, size_t>(), py::arg("reactors") = 0, py::arg("thrusters") = 0, py::arg("shields") = 0, py::arg("damage") = 0, py::arg("crew") = 0)
        .def_readwrite("reactors", &Requirements::reactors)
        .def_readwrite("thrusters", &Requirements::thrusters)
        .def_readwrite("shields", &Requirements::shields)
        .def_readwrite("damage", &Requirements::damage)
        .def_readwrite("crew", &Requirements::crew)
        .def("is_empty", &Requirements::is_empty)
        .def("to_string", &Requirements::to_string, py::arg("color") = false)
        .def("__str__", [](const Requirements& reqs) { return reqs.to_string(); })
        .def("__repr__", [](const Requirements& reqs) { return "Requirements<"+reqs.to_string()+">"; });

    py::class_<State>(m, "State")
        .def(py::init<size_t, Cards, Cards, Requirements>(), py::arg("actions"), py::arg("hand"), py::arg("draw_pile"), py::arg("requirements"))
        .def_readwrite("actions", &State::actions)
        .def_readwrite("hand", &State::hand)
        .def_readwrite("draw_pile", &State::draw_pile)
        .def_readwrite("requirements", &State::requirements);

    py::class_<Solver>(m, "Solver")
        .def(py::init<>())
        .def("explored_states_count", &Solver::explored_states_count)
        .def("get_completion_probability", &Solver::get_completion_probability, py::arg("state"));
}
/*
<%
cfg['extra_compile_args'] = ['-std=c++17', '-Wall']
setup_pybind11(cfg)
%>
*/
