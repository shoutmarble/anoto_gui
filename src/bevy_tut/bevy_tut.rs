use bevy::{prelude::*, window::{WindowMode, WindowResolution}};

pub fn bevy_tut() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Anoto GUI - Resizable Window".to_string(),
                resolution: get_window_resolution(),
                resizable: true,
                mode: WindowMode::Windowed,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, change_image_size)
        .run();
}
fn get_window_resolution() -> WindowResolution {
    // For now, we'll use reasonable defaults that work well on most modern displays
    // In a production app, you might want to query the actual monitor size at runtime
    // using winit's monitor detection, but this requires additional setup

    // Common modern resolutions: 1920x1080, 2560x1440, 3840x2160
    // We'll use 1920x1080 as a reasonable default and calculate 2/3
    let screen_width: u32 = 800;
    let screen_height: u32 = 400;

    WindowResolution::new(screen_width, screen_height)
}



fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
  // For now, we'll use reasonable defaults that work well on most modern displays
    // In a production app, you might want to query the actual monitor size at runtime
    // using winit's monitor detection, but this requires additional setup



    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::WHITE),
            ..Default::default()
        },
    ));
    
    // Spawn a simple colored rectangle for now
    commands.spawn((
        Sprite {
            // color: Color::srgb(0.8, 0.4, 0.2),
            custom_size: Some(Vec2::new(100.0, 100.0)),
            image: asset_server.load("goldfishai.png"),
            ..Default::default()
        },
        Transform::default(),
    ));

    // let image_handle = asset_server.load("goldfishai.png");
    // let mut sprite = Sprite::from_image(image_handle);
    // sprite.custom_size = Some(Vec2::new(50.0, 50.0));
    // commands.spawn(sprite);
}

fn print_image_size(sprites: Query<&Sprite>, images: Res<Assets<Image>>) {
    for sprite in sprites.iter() {
        if let Some(image) = images.get(&sprite.image) {
            println!("Image size: {}", image.size());
        }
    }
}

fn change_image_size(mut sprites: Query<&mut Sprite>) {
    let mut sprite = sprites.single_mut().unwrap();

    sprite.custom_size = Some((25.0, 25.0).into());

    
}
