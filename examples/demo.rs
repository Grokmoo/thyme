use std::collections::HashMap;

use glium::glutin::{
    self,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder
};
use glium::{Display, Surface};

use thyme::{Frame};

/// A simple party creator and character sheet for an RPG.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // load assets
    let font_src = include_bytes!("data/fonts/Roboto-Medium.ttf");
    let image_src = include_bytes!("data/images/gui.png");
    let image = image::load_from_memory(image_src).unwrap().to_rgba();
    let theme_src = include_str!("data/theme.yml");
    let theme: serde_yaml::Value = serde_yaml::from_str(theme_src)?;
    let window_size = [1280.0, 720.0];

    // create glium display
    let event_loop = EventLoop::new();
    let context = glutin::ContextBuilder::new();
    let builder = WindowBuilder::new()
        .with_title("Thyme Demo")
        .with_inner_size(glutin::dpi::LogicalSize::new(window_size[0], window_size[1]));
    let display = Display::new(builder, context, &event_loop)?;

    // create thyme backend
    let mut io = thyme::WinitIo::new();
    let mut renderer = thyme::GliumRenderer::new(&display)?;
    let mut context_builder = thyme::ContextBuilder::new(theme, &mut renderer, &mut io)?;

    // register resources in thyme and create the context
    let image_dims = image.dimensions();
    context_builder.register_texture("gui", &image.into_raw(), image_dims)?;
    context_builder.register_font_source("roboto", font_src.to_vec())?;
    let mut context = context_builder.build(window_size.into())?;

    let mut party = Party::default();

    // run main loop
    event_loop.run(move |event, _, control_flow| match event {
        Event::MainEventsCleared => {
            let gl_window = display.gl_window();
            gl_window.window().request_redraw();
        }
        Event::RedrawRequested(_) => {
            let mut target = display.draw();
            target.clear_color(0.0, 0.0, 0.0, 0.0);

            let mut frame = context.create_frame();

            build_ui(&mut frame, &mut party);

            renderer.draw_frame(&mut target, frame).unwrap();

            target.finish().unwrap();
        }
        Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
        event => {
            io.handle_event(&mut context, &event);
        }
    })
}

#[derive(Default)]
struct Party {
    members: [Option<Character>; 4],
    editing_index: Option<usize>,
}

#[derive(Default)]
struct Character {
    stats: HashMap<Stat, u32>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum Stat {
    Strength,
    Dexterity,
    Constitution,
    Intelligence,
    Wisdom,
    Charisma,
}

impl Stat {
    fn iter() -> impl Iterator<Item=Stat> + 'static {
        use Stat::*;
        [Strength, Dexterity, Constitution, Intelligence, Wisdom, Charisma].iter().copied()
    }
}

/// The function to build the Thyme user interface.  Called once each frame.  This
/// example demonstrates a combination of Rust layout and styling as well as use
/// of the theme definition file, loaded above
fn build_ui(ui: &mut Frame, party: &mut Party) {

    ui.window("party_window", "party_window", |ui| {
        ui.start("members_panel")
        .children(|ui| {
            let mut add_character_shown = false;
            for (index, member) in party.members.iter_mut().enumerate() {
                match member.as_mut() {
                    None => {
                        if add_character_shown {
                            ui.start("empty_slot_button").enabled(false).finish();
                        } else {
                            add_character_shown = true;
                            if ui.start("add_character_button").finish().clicked {
                                *member = Some(Character::default());
                                ui.set_open("character_window", true);
                                party.editing_index = Some(index);
                            }
                        }
                    },
                    Some(_member) => {
                        let clicked = ui.start("filled_slot_button")
                        .active(Some(index) == party.editing_index)
                        .finish().clicked;

                        if clicked {
                            party.editing_index = Some(index);
                        }
                    }
                }
            }
        });
    });

    // TODO make this window modal
    if let Some(index) = party.editing_index {
        ui.window("character_window", "character_window", |ui| {
            let character = party.members[index].as_mut().unwrap();

            ui.start("stats_panel")
            .children(|ui| {
                for stat in Stat::iter() {
                    let value = character.stats.entry(stat).or_insert(10);
    
                    ui.start("stat_panel")
                    .children(|ui| {
                        ui.label("label", format!("{:?}", stat));

                        if ui.button("decrease", "-").clicked {
                            *value = 3.max(*value - 1);
                        }

                        ui.label("value", format!("{}", *value));
                        
                        if ui.button("increase", "+").clicked {
                            *value = 18.min(*value + 1);
                        }
                    }); 
                }
            });            
        });
    }
}