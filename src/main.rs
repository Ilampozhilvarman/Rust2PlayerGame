use macroquad::{input::KeyCode::U, prelude::*};

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
    grav: Gravity,
    moving: bool,
}

enum Gravity {
    Up,
    Down,
}

fn swap_move(player: &mut Player, sizes: &[f32; 2], radius: &f32, speed: &f32) {
    match player.grav {
        Gravity::Up => {
            let limit: f32 = sizes[1] - radius;
            if !(player.y >= limit) {
                player.y += speed * get_frame_time();
            }
        }
        Gravity::Down => {
            let limit: f32 = *radius;
            if !(player.y <= limit) {
                player.y -= speed * get_frame_time();
            }
        }
    };
}

#[macroquad::main(window_conf)]
async fn main() {
    //Constants
    let sizes: [f32; 2] = [screen_width(), screen_height()]; //Constant at run-time
    const PLAYER_SWAP_KEYS: [KeyCode; 2] = [KeyCode::W, KeyCode::Up];
    const PLAYER_MOVE_KEYS: [KeyCode; 4] = [KeyCode::A, KeyCode::D, KeyCode::Left, KeyCode::Right];
    const SPEED: f32 = 300.0;
    const DOWN_SPEED: f32 = 500.0;
    const COLOR: Color = GREEN;
    const RADIUS: f32 = 30.0;

    let mut player1_grav: Gravity = Gravity::Up;
    let mut player2_grav: Gravity = Gravity::Down;

    let mut player1 = Player {
        x: sizes[0] / 2.0,
        y: 0.0,
        radius: RADIUS,
        grav: Gravity::Up,
        moving: false,
    };
    let mut player2 = Player {
        x: sizes[0] / 2.0,
        y: sizes[1] - RADIUS,
        radius: RADIUS,
        grav: Gravity::Down,
        moving: false,
    };

    loop {
        //player1 movement side-to-side
        if is_key_down(PLAYER_MOVE_KEYS[0]) {
            player1.x -= SPEED * get_frame_time();
        }
        if is_key_down(PLAYER_MOVE_KEYS[1]) {
            player1.x += SPEED * get_frame_time();
        }
        //player2 movement side-to-side
        if is_key_down(PLAYER_MOVE_KEYS[2]) {
            player2.x -= SPEED * get_frame_time();
        }
        if is_key_down(PLAYER_MOVE_KEYS[3]) {
            player2.x += SPEED * get_frame_time();
        }

        //swap movement
        //player 1
        if is_key_pressed(PLAYER_SWAP_KEYS[0]) {
            if !player1.moving {
                player1.moving = true;
            }
        }

        //player 2
        if is_key_pressed(PLAYER_SWAP_KEYS[1]) {
            if !player2.moving {
                player2.moving = true;
            }
        }

        //Detects swap and makes both players go up/down:
        if player1.moving {
            swap_move(&mut player1, &sizes, &RADIUS, &DOWN_SPEED);
        }

        if player2.moving {
            swap_move(&mut player2, &sizes, &RADIUS, &DOWN_SPEED);
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
