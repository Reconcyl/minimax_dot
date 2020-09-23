use ai::{State, strategy};
use strategy::DotStrategy as _;
use strategy::PlacerStrategy as _;

use std::io::{self, Write};

fn clear_screen() {
    print!("\x1b[H\x1b[2J");
}

pub fn main() -> io::Result<()> {
    use strategy as s;
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    let mut rng = rand::thread_rng();
    let mut state = State::new(&mut rng);

    let mut placer_strategy = s::PlacerPredictive::new(rng, s::SmartPathfind);
    let mut dot_strategy = s::SmartPathfind;

    loop {
        // perform placer actions
        std::thread::sleep(
            std::time::Duration::from_millis(200));
        clear_screen();
        state.display(&mut stdout)?;

        if let Some(s) = placer_strategy.play(state) {
            state = s;
        } else {
            stdout.write_all(b"placer\n")?;
            stdout.flush()?;
            break
        }

        // perform dot actions
        std::thread::sleep(
            std::time::Duration::from_millis(200));
        clear_screen();
        state.display(&mut stdout)?;

        if let Some(s) = dot_strategy.play(state) {
            state = s;
        } else {
            stdout.write_all(b"dot\n")?;
            stdout.flush()?;
            break;
        }
    }

    Ok(())
}

