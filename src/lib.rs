use bevy::text::{scale_value, TextLayoutInfo};
use bevy::ui::RenderUiSystem;
use bevy::utils::HashSet;
use bevy::window::WindowScaleFactorChanged;
use bevy::{prelude::*, text::TextPipeline};
use bevy::{
    render::view::{check_visibility, VisibilitySystems},
    ui::ExtractedUiNodes,
};
use bevy::{render::Extract, text::TextSettings};
use bevy::{render::RenderApp, text::FontAtlasSets};
use bevy::{text::PositionedGlyph, ui::ExtractedUiNode};
use bevy::{text::Text2dBounds, window::PrimaryWindow};
use bevy::{text::YAxisOrientation, ui::NodeType};

/// Newtype wrapper for [`Text`]
///
/// Required so that the text isn't also extracted by `extract_text2d_sprite`
/// and consequently drawn twice.
#[derive(Clone, Component, Default, Debug, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub struct UiText(pub Text);

impl From<Text> for UiText {
    fn from(text: Text) -> Self {
        Self(text)
    }
}

impl UiText {
    /// Constructs a [`UiText`] with a single section.
    ///
    /// See [`Text`] for more details
    pub fn from_section(value: impl Into<String>, style: TextStyle) -> Self {
        Self(Text::from_section(value, style))
    }

    /// Constructs a [`UiText`] from a list of sections.
    ///
    /// See [`Text`] for more details
    pub fn from_sections(sections: impl IntoIterator<Item = TextSection>) -> Self {
        Self(Text::from_sections(sections))
    }

    /// Appends a new text section to the end of the text.
    pub fn push_section(&mut self, value: impl Into<String>, style: TextStyle) {
        self.sections.push(TextSection {
            value: value.into(),
            style,
        });
    }
}

/// Bundle of components needed to draw text to the Bevy UI
/// at any position and depth
#[derive(Bundle, Default)]
pub struct IndependentTextBundle {
    pub text: UiText,
    pub text_2d_bounds: Text2dBounds,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
    pub text_layout: TextLayoutInfo,
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_ui_independent_text_layout(
    mut queue: Local<HashSet<Entity>>,
    mut textures: ResMut<Assets<Image>>,
    fonts: Res<Assets<Font>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut scale_factor_changed: EventReader<WindowScaleFactorChanged>,
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    mut font_atlas_set_storage: ResMut<FontAtlasSets>,
    mut text_pipeline: ResMut<TextPipeline>,
    text_settings: Res<TextSettings>,
    mut text_query: Query<(
        Entity,
        Ref<UiText>,
        Option<&Text2dBounds>,
        &mut TextLayoutInfo,
    )>,
) {
    let factor_changed = scale_factor_changed.read().last().is_some();
    let scale_factor = match windows.get_single() {
        Ok(window) => window.scale_factor(),
        Err(_) => return,
    };
    for (entity, ui_text, maybe_bounds, mut layout) in &mut text_query {
        let UiText(text) = ui_text.as_ref();
        if factor_changed || ui_text.is_changed() || queue.remove(&entity) {
            let text_bounds = match maybe_bounds {
                Some(bounds) => Vec2::new(
                    scale_value(bounds.size.x, scale_factor),
                    scale_value(bounds.size.y, scale_factor),
                ),
                None => Vec2::new(f32::MAX, f32::MAX),
            };
            match text_pipeline.queue_text(
                &fonts,
                &text.sections,
                scale_factor,
                text.justify,
                text.linebreak_behavior,
                text_bounds,
                &mut font_atlas_set_storage,
                &mut texture_atlases,
                &mut textures,
                &text_settings,
                YAxisOrientation::TopToBottom,
            ) {
                Err(TextError::NoSuchFont) => {
                    queue.insert(entity);
                }
                Err(e @ TextError::FailedToAddGlyph(_)) => {
                    panic!("Fatal error when processing text: {}.", e);
                }
                Ok(text_layout_info) => {
                    layout.logical_size = Vec2::new(
                        scale_value(text_layout_info.logical_size.x, 1. / scale_factor),
                        scale_value(text_layout_info.logical_size.y, 1. / scale_factor),
                    );
                    *layout = text_layout_info
                }
            }
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn extract_text_sprite(
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    texture_atlases: Extract<Res<Assets<TextureAtlasLayout>>>,
    mut commands: Commands,
    default_ui_camera: Extract<DefaultUiCamera>,
    camera_query: Extract<Query<(Entity, &Camera)>>,
    text_query: Extract<
        Query<(
            &GlobalTransform,
            &UiText,
            &ViewVisibility,
            &TextLayoutInfo,
            Option<&TargetCamera>,
        )>,
    >,
) {
    for (global_transform, text, computed_visibility, text_layout, maybe_camera) in
        text_query.iter()
    {
        if !computed_visibility.get() {
            continue;
        }

        let Some(camera_entity) = maybe_camera
            .map(TargetCamera::entity)
            .or(default_ui_camera.get())
        else {
            continue;
        };

        let scale_factor = camera_query
            .get(camera_entity)
            .ok()
            .and_then(|(_, c)| c.target_scaling_factor())
            .unwrap_or(1.0);
        let inverse_scale_factor = scale_factor.recip();

        let text_glyphs = &text_layout.glyphs;
        let (width, height) = (text_layout.logical_size.x, text_layout.logical_size.y);
        let alignment_offset = -Vec2::new(width, height) * (Vec2::splat(0.5));

        let mut transform = global_transform.affine()
            * bevy::math::Affine3A::from_translation(alignment_offset.extend(0.));

        transform.translation *= scale_factor;
        transform.translation = transform.translation.round();
        transform.translation *= inverse_scale_factor;

        let mut color = LinearRgba::from(Color::WHITE);
        let mut current_section = usize::MAX;
        for PositionedGlyph {
            position,
            atlas_info,
            section_index,
            ..
        } in text_glyphs
        {
            if *section_index != current_section {
                color = LinearRgba::from(text.sections[*section_index].style.color);
                current_section = *section_index;
            }
            let atlas = texture_atlases.get(&atlas_info.texture_atlas).unwrap();

            let mut rect = atlas.textures[atlas_info.glyph_index].as_rect();
            rect.min *= inverse_scale_factor;
            rect.max *= inverse_scale_factor;

            let extracted_transform = global_transform.compute_matrix()
                * Mat4::from_scale(Vec3::splat(scale_factor.recip()))
                * Mat4::from_translation(
                    alignment_offset.extend(0.) * scale_factor + position.extend(0.),
                );
            extracted_uinodes.uinodes.insert(
                commands.spawn_empty().id(),
                ExtractedUiNode {
                    stack_index: global_transform.translation().z as u32,
                    transform: extracted_transform,
                    color,
                    rect,
                    image: atlas_info.texture.id(),
                    atlas_size: Some(atlas.size.as_vec2() * inverse_scale_factor),
                    clip: None,
                    flip_x: false,
                    flip_y: false,
                    camera_entity,
                    border: [0.; 4],
                    border_radius: [0.; 4],
                    node_type: NodeType::Rect,
                },
            );
        }
    }
}

pub struct IndependentTextPlugin;

impl Plugin for IndependentTextPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<UiText>()
            .add_systems(PostUpdate, update_ui_independent_text_layout)
            .add_systems(
                PostUpdate,
                check_visibility::<With<UiText>>.in_set(VisibilitySystems::CheckVisibility),
            );
        let render_app = match app.get_sub_app_mut(RenderApp) {
            Some(render_app) => render_app,
            None => return,
        };
        render_app.add_systems(
            ExtractSchedule,
            extract_text_sprite.after(RenderUiSystem::ExtractText),
        );
    }
}
