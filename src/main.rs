use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "My Fullscreen Game".to_owned(),
        fullscreen: true,
        ..Default::default()
    }
}

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
    radius: f32,
}

#[macroquad::main(window_conf)]
async fn main() {
    let sizes: [f32; 2] = [screen_width(), screen_height()];
    const PLAYER_SWAP_KEYS: [KeyCode; 2] = [KeyCode::W, KeyCode::Up];
    const PLAYER_MOVE_KEYS: [KeyCode; 4] = [KeyCode::A, KeyCode::D, KeyCode::Left, KeyCode::Right];
    const SPEED: f32 = 300.0;
    const COLOR: Color = GREEN;
    const RADIUS: f32 = 30.0;
    let mut player1 = Player {
        x: sizes[0] / 2.0,
        y: 0.0,
        radius: RADIUS,
    };
    let mut player2 = Player {
        x: sizes[0] / 2.0,
        y: sizes[1] - RADIUS,
        radius: RADIUS,
    };

    loop {
        //player1 movement:
        if is_key_down(PLAYER_MOVE_KEYS[0]) {
            player1.x -= SPEED * get_frame_time();
        }
        if is_key_down(PLAYER_MOVE_KEYS[1]) {
            player1.x += SPEED * get_frame_time();
        }
        //player2 movement
        if is_key_down(PLAYER_MOVE_KEYS[2]) {
            player2.x -= SPEED * get_frame_time();
        }
        if is_key_down(PLAYER_MOVE_KEYS[3]) {
            player2.x += SPEED * get_frame_time();
        }
        clear_background(BLACK);
        player1.x = player1.x.clamp(RADIUS, sizes[0] - RADIUS);
        player1.y = player1.y.clamp(RADIUS, sizes[1] - RADIUS);
        player2.x = player2.x.clamp(RADIUS, sizes[0] - RADIUS);
        player2.y = player2.y.clamp(RADIUS, sizes[1] - RADIUS);
        draw_circle(player1.x, player1.y, player1.radius, COLOR);
        draw_circle(player2.x, player2.y, player2.radius, COLOR);
        next_frame().await;
    }
}
