use std::collections::HashMap;

use glium::glutin::{
    self,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder
};
use glium::{Display, Surface};

use thyme::{Frame, Align, bench};

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

    // hide the default cursor
    display.gl_window().window().set_cursor_visible(false);

    // create thyme backend
    let mut io = thyme::WinitIo::new(&event_loop, window_size.into());
    let mut renderer = thyme::GliumRenderer::new(&display)?;
    let mut context_builder = thyme::ContextBuilder::new(theme, &mut renderer, &mut io)?;

    // register resources in thyme and create the context
    let image_dims = image.dimensions();
    context_builder.register_texture("gui", &image.into_raw(), image_dims)?;
    context_builder.register_font_source("roboto", font_src.to_vec())?;
    let mut context = context_builder.build()?;

    let mut party = Party::default();

    // run main loop
    event_loop.run(move |event, _, control_flow| match event {
        Event::MainEventsCleared => {
            let frame_start = std::time::Instant::now();

            let mut target = display.draw();
            target.clear_color(0.0, 0.0, 0.0, 0.0);

            let thyme_bench = bench::start("thyme");

            let frame_bench = bench::start("frame");

            let mut frame = context.create_frame();

            // show a custom cursor.  it automatically inherits mouse presses in its state
            frame.set_mouse_cursor("gui/cursor", Align::TopLeft);
            build_ui(&mut frame, &mut party);

            bench::end(frame_bench);

            let draw_bench = bench::start("draw");
            renderer.draw_frame(&mut target, frame).unwrap();
            bench::end(draw_bench);

            bench::end(thyme_bench);

            target.finish().unwrap();

            *control_flow = ControlFlow::WaitUntil(frame_start + std::time::Duration::from_millis(16));
        }
        Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
        event => {
            io.handle_event(&mut context, &event);
        }
    })
}

#[derive(Default)]
struct Party {
    members: Vec<Character>,
    editing_index: Option<usize>,
}

const INITIAL_GP: u32 = 100;
const MIN_STAT: u32 = 3;
const MAX_STAT: u32 = 18;
const STAT_POINTS: u32 = 75;

struct Character {
    name: String,
    stats: HashMap<Stat, u32>,

    race: Race,
    gp: u32,
    items: Vec<Item>,
}

impl Character {
    fn generate(index: usize) -> Character {
        Character {
            name: format!("Charname {}", index),
            stats: HashMap::default(),
            gp: INITIAL_GP,
            items: Vec::default(),
            race: Race::default(),
        }
    }
}

#[derive(Debug)]
enum Race {
    Human,
    Elf,
    Dwarf,
    Halfling,
}

impl Default for Race {
    fn default() -> Self {
        Race::Human
    }
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

#[derive(Clone)]
struct Item {
    name: &'static str,
    price: u32,
}

const ITEMS: [Item; 3] = [
    Item { name: "Sword", price: 50 },
    Item { name: "Shield", price: 20 },
    Item { name: "Torch", price: 2 }
];

/// The function to build the Thyme user interface.  Called once each frame.  This
/// example demonstrates a combination of Rust layout and styling as well as use
/// of the theme definition file, loaded above
fn build_ui(ui: &mut Frame, party: &mut Party) {
    ui.label("bench", format!(
        "{}\n{}\n{}",
        bench::report("thyme"),
        bench::report("frame"),
        bench::report("draw"),
    ));

    ui.window("party_window", "party_window", |ui| {
        ui.scrollpane("members_panel", "party_content", |ui| {
            party_members_panel(ui, party);
        });
    });

    if let Some(index) = party.editing_index {
        let character = &mut party.members[index];

        ui.window("character_window", "character_window", |ui| {
            // TODO scroll with mouse wheel
            ui.scrollpane("pane", "character_content", |ui| {
                ui.start("name_panel")
                .children(|ui| {
                    if let Some(new_name) = ui.input_field("name_input", "name_input") {
                        character.name = new_name;
                    }
                });

                ui.gap(10.0);

                // TODO combo box
                ui.button("race_selector", format!("{:?}", character.race));
    
                ui.gap(10.0);
    
                ui.start("stats_panel")
                .children(|ui| {
                    stats_panel(ui, character);
                });
                
                ui.gap(10.0);
    
                ui.start("inventory_panel")
                .children(|ui| {
                    inventory_panel(ui, character);
                });
            });
        });

        // TODO window draw order
        ui.window("item_picker", "item_picker", |ui| {
            item_picker(ui, character);
        });
    }
}

fn party_members_panel(ui: &mut Frame, party: &mut Party) {
    for (index, member) in party.members.iter_mut().enumerate() {
        let clicked = ui.start("filled_slot_button")
        .text(&member.name)
        .active(Some(index) == party.editing_index)
        .finish().clicked;

        if clicked {
            set_active_character(ui, member);
            party.editing_index = Some(index);
        }
    }

    if ui.start("add_character_button").finish().clicked {
        let new_member = Character::generate(party.members.len());
        set_active_character(ui, &new_member);
        party.members.push(new_member);
        party.editing_index = Some(party.members.len() - 1);
    }
}

fn set_active_character(ui: &mut Frame, character: &Character) {
    ui.open("character_window");
    ui.modify("name_input", |state| {
        state.text = Some(character.name.clone());
    });
    ui.close("item_picker");
}

fn stats_panel(ui: &mut Frame, character: &mut Character) {
    let points_used: u32 = character.stats.values().sum();
    let points_available: u32 = STAT_POINTS - points_used;

    ui.child("title");

    for stat in Stat::iter() {
        let value = character.stats.entry(stat).or_insert(10);

        ui.start("stat_panel")
        .children(|ui| {
            ui.label("label", format!("{:?}", stat));

            let clicked = ui.start("decrease").enabled(*value > MIN_STAT).finish().clicked;
            if clicked {
                *value -= 1;
            }

            ui.label("value", format!("{}", *value));
            
            let clicked = ui.start("increase").enabled(points_available > 0 && *value < MAX_STAT).finish().clicked;
            if clicked {
                *value = 18.min(*value + 1);
            }
        }); 
    }

    ui.label("points_available", format!("Points Remaining: {}", points_available));
}

fn item_picker(ui: &mut Frame, character: &mut Character) {
    for item in ITEMS.iter() {
        let clicked = ui.start("item_button")
        .enabled(character.gp >= item.price)
        .children(|ui| {
            ui.label("name", item.name);
            // TODO icon image
            ui.child("icon");
            ui.label("price", format!("{} Gold", item.price));
        }).clicked;

        if clicked {
            character.gp -= item.price;
            character.items.push(item.clone());
            ui.close("item_picker");
        }
    }
}

fn inventory_panel(ui: &mut Frame, character: &mut Character) {
    ui.child("title");
    ui.start("top_panel")
    .children(|ui| {
        if ui.child("buy").clicked {
            ui.open_modal("item_picker");
        }

        ui.label("gold", format!("{} Gold", character.gp));
    });
    
    ui.scrollpane("items_panel", "items_content", |ui| {
        items_panel(ui, character);
    });
}

fn items_panel(ui: &mut Frame, character: &mut Character) {
    let mut sell = None;
    for (index, item) in character.items.iter().enumerate() {
        // TODO tooltip that says remove item
        if ui.button("item_button", item.name).clicked {
            sell = Some(index);
        }
    }

    if let Some(index) = sell {
        let item = character.items.remove(index);
        character.gp += item.price;
    }
}