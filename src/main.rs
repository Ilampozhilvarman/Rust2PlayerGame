use macroquad::prelude::*;

struct Object {
    x: f32,
    y: f32,
}

struct Circle {
    obj: Object,
    radius: f32,
}

struct Wall {
    obj: Object,
    height: f32,
    width: f32,
}

struct Player {
    x: f32,
    y: f32,
}

#[macroquad::main("WASD Example")]
async fn main() {
    let sizes: [f32; 2] = [screen_width(), screen_height()];
    const player_swap_keys: [KeyCode; 2] = [KeyCode::W, KeyCode::Up];
    const player_move_keys: [KeyCode; 4] = [KeyCode::A, KeyCode::D, KeyCode::Left, KeyCode::Right];
    const speed: f32 = 300.0;
    let mut player1 = Player {
        x: sizes[0] / 2,
        y: 0.0,
    };
    let mut player2 = Player {
        x: sizes[0] / 2,
        y: sizes[1],
    };

    loop {
        //player1 movement:
        if is_key_down(player_move_keys[0]) {
            player1.x -= speed * get_frame_time();
        }
        if is_any_key_down(player_move_keys[2]) {
            player1.x += speed * get_frame_time();
        }
        clear_background(LIGHTGRAY);

        next_frame().await;
    }
}
