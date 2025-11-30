use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::asset::RenderAssetUsages;
use image::{DynamicImage, GenericImageView};
use rfd::FileDialog;

use crate::gui_app::{
    layout::LoadImageButton,
    state::{GuiImageState, ImageLoadedEvent},
};

pub struct LoaderPlugin;

impl Plugin for LoaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_load_image_button);
    }
}

fn handle_load_image_button(
    mut buttons: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<LoadImageButton>)>,
    mut images: ResMut<Assets<Image>>,
    mut gui_image: ResMut<GuiImageState>,
    mut loaded_writer: MessageWriter<ImageLoadedEvent>,
) {
    for (interaction, mut color) in &mut buttons {
        match *interaction {
            Interaction::Pressed => {
                *color = BackgroundColor(Color::srgb(0.6, 0.4, 1.0));
                if let Some(path) = FileDialog::new().add_filter("image", &["png", "jpg", "jpeg"]).pick_file() {
                    if let Ok(dynamic) = image::open(&path) {
                        if let Some(handle) = push_dynamic_image(&dynamic, &mut images) {
                            gui_image.set_image(dynamic, handle.clone());
                            loaded_writer.write(ImageLoadedEvent { handle });
                        }
                    }
                }
            }
            Interaction::Hovered => {
                *color = BackgroundColor(Color::srgb(0.55, 0.35, 0.95));
            }
            Interaction::None => {
                *color = BackgroundColor(Color::srgb(0.4, 0.2, 0.8));
            }
        }
    }
}

pub(crate) fn push_dynamic_image(dynamic: &DynamicImage, images: &mut Assets<Image>) -> Option<Handle<Image>> {
    let rgba = dynamic.to_rgba8();
    let (width, height) = dynamic.dimensions();
    let extent = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let mut image = Image::new(
        extent,
        TextureDimension::D2,
        rgba.into_raw(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;
    Some(images.add(image))
}
