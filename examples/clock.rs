use std::{
    sync::LazyLock,
    time::{Duration, Instant},
};

use pancurses::{
    Attributes, COLOR_BLACK, COLOR_GREEN, COLOR_PAIR, COLOR_RED, Input, Window, curs_set, endwin,
    init_pair, initscr, noecho, start_color,
};
use time_control::{
    AnyClock, AnyConfig, BronsteinConfig, ByoYomiConfig, FischerConfig, SimpleDelayConfig,
    TimeControl, TwoPlayer,
};

type BoxedGameClock = TwoPlayer<AnyClock>;

fn main() {
    let win = initscr();
    start_color();
    noecho();
    curs_set(0);

    init_pair(1, COLOR_RED, COLOR_BLACK);
    init_pair(2, COLOR_GREEN, COLOR_BLACK);

    win.printw("Hello!");
    win.refresh();
    win.keypad(true);
    win.nodelay(true);

    loop {
        let Some(choice) = choose_time_control(&win) else {
            break;
        };
        let cf = (choice.make)();
        let tc = TwoPlayer::new(cf);
        run_time_control(&win, choice.label, tc);
    }

    endwin();
}

struct Choice {
    label: &'static str,
    make: fn() -> AnyConfig,
}

static CHOICES: LazyLock<Vec<Choice>> = LazyLock::new(|| {
    vec![
        Choice {
            label: "Simple delay 3:00 (5s)",
            make: || {
                AnyConfig::from(SimpleDelayConfig {
                    initial: Duration::from_mins(3),
                    delay: Duration::from_secs(5),
                })
            },
        },
        Choice {
            label: "Fischer 3:00 +5s",
            make: || {
                AnyConfig::from(FischerConfig {
                    initial: Duration::from_mins(3),
                    increment: Duration::from_secs(5),
                    limit: None,
                })
            },
        },
        Choice {
            label: "Bronstein 3:00 +5s",
            make: || {
                AnyConfig::from(BronsteinConfig {
                    initial: Duration::from_mins(3),
                    max_increment: Duration::from_secs(5),
                })
            },
        },
        Choice {
            label: "Byo-yomi 2:00 +3x20s",
            make: || {
                AnyConfig::from(ByoYomiConfig {
                    initial: Duration::from_mins(2),
                    period_time: Duration::from_secs(20),
                    num_periods: 3,
                })
            },
        },
    ]
});

fn choose_time_control(win: &Window) -> Option<&'static Choice> {
    win.clear();
    win.nodelay(false);

    let mut choice = 0;
    loop {
        win.mvprintw(2, 2, "Choose a time control:");
        for (i, x) in CHOICES.iter().enumerate() {
            win.mv(4 + i as i32, 2);
            match i == choice {
                true => win.printw("-> "),
                false => win.printw("   "),
            };
            win.printw(x.label);
        }
        win.mvprintw(
            5 + CHOICES.len() as i32,
            2,
            "Arrows:Select Enter:Choose q:Exit",
        );

        match win.getch() {
            Some(Input::KeyUp) => choice = choice.saturating_sub(1),
            Some(Input::KeyDown) => choice = (choice + 1).min(CHOICES.len() - 1),
            Some(Input::KeyEnter) | Some(Input::Character('\n')) | Some(Input::Character('\r')) => {
                return Some(&CHOICES[choice]);
            }
            Some(Input::Character('q')) => return None,
            _ => (),
        }
    }
}

fn run_time_control(win: &Window, name: &'static str, mut tc: BoxedGameClock) {
    win.clear();
    win.nodelay(true);

    let mut now = Instant::now();
    let mut last_update = now;
    let mut ffw = false;
    let mut paused = false;

    loop {
        win.mvprintw(2, 2, name);
        win.mvprintw(4, 2, "P1: ");
        win.clrtoeol();
        show_time_control(win, tc.p1(), tc.p1_turn());
        win.mvprintw(5, 2, "P2: ");
        win.clrtoeol();
        show_time_control(win, tc.p2(), tc.p2_turn());
        win.mvprintw(7, 2, "Space:Toggle p:Pause f:Fast-forward r:Reset q:Exit");

        match win.getch() {
            Some(Input::Character(' ')) => match tc.p1_turn() {
                true => tc.set_p2_turn(),
                false => tc.set_p1_turn(),
            },
            Some(Input::Character('r')) => {
                tc.reset();
                paused = false;
                ffw = false;
            }
            Some(Input::Character('p')) => paused = !paused,
            Some(Input::Character('f')) => ffw = !ffw,
            Some(Input::Character('q')) => return,
            _ => (),
        }

        if !paused {
            let mut elapsed = now - last_update;
            if ffw {
                elapsed = elapsed * 20;
            }
            tc.turn_spend(elapsed);
        }
        last_update = now;

        std::thread::sleep(Duration::from_millis(10));
        now = Instant::now();
    }
}

fn show_time_control(win: &Window, tc: &AnyClock, is_turn: bool) {
    win.attrset(if tc.is_expired() {
        COLOR_PAIR(1)
    } else if is_turn {
        COLOR_PAIR(2)
    } else {
        0
    });
    win.printw(format!("{}", tc));
    win.attrset(Attributes::new());
}
