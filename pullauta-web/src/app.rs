use std::{
    io::{BufWriter, Write},
    path::PathBuf,
    sync::Arc,
};

use egui::{CollapsingHeader, Color32, ColorImage, ImageData, TextureHandle, TextureOptions};
use log::{debug, info, warn};
use pullauta::io::fs::{
    memory::{Directory, MemoryFileSystem},
    FileSystem,
};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    #[serde(skip)]
    fs: pullauta::io::fs::memory::MemoryFileSystem,
    #[serde(skip)]
    radio: PathBuf,
    #[serde(skip)]
    old_radio: PathBuf,

    #[serde(skip)]
    screen_texture: Option<TextureHandle>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            fs: Default::default(),
            radio: PathBuf::new(),
            old_radio: PathBuf::new(),
            screen_texture: None,
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        let mut s = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Self::default()
        };

        let screen_texture = cc.egui_ctx.load_texture(
            "screen",
            ImageData::Color(Arc::new(ColorImage::new([320, 80], Color32::TRANSPARENT))),
            TextureOptions::default(),
        );

        s.screen_texture = Some(screen_texture);
        s
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        egui::SidePanel::left("side_panel")
            .resizable(true)
            .show(ctx, |ui| {
                // The side panel is often a good place for tools and options.

                ui.heading("Side Panel");

                ui.label("File system:");

                if ui.button("Create directory").clicked() {
                    self.fs.create_dir_all("new_directory/deep/subdir").unwrap();
                }

                // TODO: a file system tree
                // ui.label(format!("{:?}", self.fs));
                show_file_system_tree(ui, &self.fs, &mut self.radio);

                ui.separator();

                if ui.button("Delete selected file").clicked() {
                    self.fs.remove_file(&self.radio).unwrap();
                    self.radio = PathBuf::new();
                }

                if let Some(name) = self.radio.file_name() {
                    let name = name.to_string_lossy();

                    if name.ends_with(".laz") {
                        if ui.button("Process LAZ").clicked() {
                            info!("Processing LAZ file: {:?}", self.radio);
                            // TODO: call pullauta function to process LAZ file
                            let fs = self.fs.clone();
                            let config = pullauta::config::Config::default();
                            let thread = String::new();
                            let tmpfolder = PathBuf::from(format!("temp{}", thread));
                            pullauta::process::process_tile(
                                &fs,
                                &config,
                                &thread,
                                &tmpfolder,
                                &self.radio,
                                false,
                            )
                            .expect("Failed to process LAZ file");
                        }
                    }
                }
            });

        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(true)
            .min_height(200.0)
            .show(ctx, |ui| {
                egui_logger::logger_ui().show(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            // use selected file to display more information
            ui.label(format!("Selected file: {:?}", self.radio));
            if let Ok(size) = self.fs.file_size(&self.radio) {
                ui.label(format!("File size: {}", size));
            }

            if self.radio != self.old_radio {
                self.old_radio = self.radio.clone();
                if self.fs.exists(&self.radio) {
                    if let Ok(img) = self.fs.read_image(&self.radio) {
                        if let Some(texture) = &mut self.screen_texture {
                            // upload the image data to the texture
                            texture.set(
                                ColorImage::from_rgb(
                                    [img.width() as usize, img.height() as usize],
                                    &img.to_rgb8().into_raw(),
                                ),
                                TextureOptions::default(),
                            );
                        }
                    }
                }
            }

            if let Some(texture) = &self.screen_texture {
                // TODO: how can we scae the image to fit the screen? And how can we zoom in/out
                // and pan?
                ui.image(&texture.clone());
            }
        });

        preview_files_being_dropped(ctx);

        // Collect dropped files:
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                // copy the files into the in-memory file system:
                for file in &i.raw.dropped_files {
                    debug!("Importing dropped file: {:?}", file.name);

                    if let Some(bytes) = &file.bytes {
                        let mut writer = BufWriter::new(self.fs.create(&file.name).unwrap());
                        writer.write_all(bytes).unwrap();
                    } else {
                        warn!("Dropped file has no bytes");
                    }
                }
            }
        });
    }
}

/// Recursively show the file system as a tree.
fn show_file_system_tree(ui: &mut egui::Ui, fs: &MemoryFileSystem, radio: &mut PathBuf) {
    // open fs for reading
    let root = fs.root();
    let root = root.read().unwrap();
    recursive_dir_header(ui, &root.0, PathBuf::new(), "root", 0, radio);
}

fn recursive_dir_header(
    ui: &mut egui::Ui,
    dir: &Directory,
    parent: PathBuf,
    name: &str,
    depth: usize,
    radio: &mut PathBuf,
) {
    let response = CollapsingHeader::new(name)
        .default_open(depth < 1)
        .show(ui, |ui| recursive_dir(ui, dir, parent, depth, radio));
    response.header_response.context_menu(|ui| {
        if ui.button("Delete").clicked() {
            info!("Delete {:?}", name);
        };
    });
}
fn recursive_dir(
    ui: &mut egui::Ui,
    dir: &Directory,
    parent: PathBuf,
    depth: usize,
    radio: &mut PathBuf,
) {
    // iterate all subfolder recusively
    let mut subdirs = dir.subdirs.iter().collect::<Vec<_>>();
    subdirs.sort_by(|(a, _), (b, _)| a.cmp(b));
    for (name, sub_dir) in subdirs {
        recursive_dir_header(
            ui,
            sub_dir,
            parent.clone().join(&name),
            name,
            depth + 1,
            radio,
        );
    }

    // iterate all files
    let mut files = dir.files.iter().collect::<Vec<_>>();
    files.sort_by(|(a, _), (b, _)| a.cmp(b));
    for (name, _) in files {
        ui.radio_value(radio, parent.join(&name), name);
    }
}

/// Preview hovering files:
fn preview_files_being_dropped(ctx: &egui::Context) {
    use egui::{Align2, Color32, Id, LayerId, Order, TextStyle};
    use std::fmt::Write as _;

    if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
        let text = ctx.input(|i| {
            let mut text = "Dropping files:\n".to_owned();
            for file in &i.raw.hovered_files {
                if let Some(path) = &file.path {
                    write!(text, "\n{}", path.display()).ok();
                } else if !file.mime.is_empty() {
                    write!(text, "\n{}", file.mime).ok();
                } else {
                    text += "\n???";
                }
            }
            text
        });

        let painter =
            ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

        let screen_rect = ctx.screen_rect();
        painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
        painter.text(
            screen_rect.center(),
            Align2::CENTER_CENTER,
            text,
            TextStyle::Heading.resolve(&ctx.style()),
            Color32::WHITE,
        );
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
