/*
 * Copyright (c) 2026 Thomas Prosser
 * Licensed under MIT License
 * https://opensource.org/licenses/MIT
 */

use iced::widget::overlay::menu;
use std::path::PathBuf;

use iced::widget::{text, column, center, button, pick_list, row, container, svg};
use iced::{Center, Element, Task, Color, Theme, Length, window, Background, Border, Shadow};
use iced::theme::Palette;

use swiss_qrust::{BillData, InputBill, Language};
use swiss_qrust::pdf::create_pdf;

// Define the official Swiss Red (Pantone 485 C)
const SWISS_RED: Color = Color::from_rgb(0.835, 0.168, 0.117); // #D52B1E
const SWISS_RED_HOVER: Color = Color::from_rgb(0.7, 0.1, 0.08);

const INTER_REGULAR: &[u8] = include_bytes!("../../../assets/fonts/Inter_24pt-Regular.ttf");
const INTER_BOLD: &[u8] = include_bytes!("../../../assets/fonts/Inter_28pt-Bold.ttf");


#[derive(Default)]
struct SwissQrApp {
    status_message: String,
    selected_language: Language,
}

#[derive(Debug, Clone)]
enum Message {
    OpenPicker,
    LanguageChanged(Language),
    FileSelected(Option<PathBuf>),
    PdfGenerated(Result<PathBuf, String>),
}
impl SwissQrApp {
    // Logic remains mostly the same, but no longer tied to a trait
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::LanguageChanged(lang) => {
                self.selected_language = lang;
                Task::none()
            },
            Message::OpenPicker => {
                Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("Config Files", &["toml", "json"])
                            .set_title("Select Bill Configuration")
                            .pick_file()
                            .await
                            .map(|handle| handle.path().to_path_buf())
                    },
                    Message::FileSelected,
                )
            },

            Message::FileSelected(Some(path)) => {
                self.status_message = format!("Processing: {:?}", path.file_name().unwrap());
                // Task to run your PDF logic in the background
                Task::perform(generate_pdf(path, self.selected_language), Message::PdfGenerated)
            },

            Message::FileSelected(None) => {
                self.status_message = "Selection cancelled.".into();
                Task::none()
            },

            Message::PdfGenerated(Ok(saved_path)) => {
                self.status_message = format!("Success! Saved to: {:?}", saved_path);
                Task::none()
            }

            Message::PdfGenerated(Err(e)) => {
                self.status_message = format!("Error: {}", e);
                Task::none()
            }

        }
    }

    fn view(&self) -> Element<'_, Message> {

        let bold = iced::Font::with_name("Inter");

        let languages: &[Language] = &[
            Language::De,
            Language::Fr,
            Language::It,
            Language::En,
            Language::Rm
        ];

        center(
            column![
                row![
                     svg(svg::Handle::from_memory(
                        include_bytes!("../../../assets/svg/CH-Kreuz-Rot.svg")
                    ))
                        .width(50.0)
                        .height(50.0),
                    column![
                        text("Swiss QR").size(40).font(bold),
                        text("Generator").size(24).font(bold).color(SWISS_RED),
                    ]
                ].spacing(10),
                container(
                    row![
                        text("Language:"),
                        pick_list(languages, Some(&self.selected_language), Message::LanguageChanged)
                        .placeholder("Select language".to_string())
                        .style(swiss_picklist)
                        .menu_style(swiss_menu)
                        .width(Length::Shrink),
                    ]
                    .spacing(10)
                    .padding(20).align_y(Center),
                ).style(container::bordered_box),

                button(text("Pick .toml or .json file").size(18).font(bold))
                .padding(15)
                .style(primary_button)
                .on_press(Message::OpenPicker),
                text(&self.status_message).size(12).color([0.4, 0.4, 0.4]),
            ]
                .spacing(20)
                .align_x(Center)
        ).padding(40)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }
}

fn main() -> iced::Result {
    // Instead of Sandbox::run, we use the application builder
    iced::application(SwissQrApp::default, SwissQrApp::update, SwissQrApp::view)
        .title("Swiss QR Bill GUI")
        .font(INTER_REGULAR)
        .font(INTER_BOLD)
        .window(window::Settings {
            size: (500.0, 400.0).into(), // Custom window size
            ..Default::default()
        })
        .theme(swiss_theme())
        .run()
}

async fn generate_pdf(input_path: PathBuf, language: Language) -> Result<PathBuf, String> {
    // Calling a dedicated private helper keeps the async wrapper clean
    let output_path = input_path.with_extension("pdf");
    process_bill(&input_path, &output_path, language).map_err(|e| format!("{e:#}"))?;
    Ok(output_path)
}

fn process_bill(in_path: &PathBuf, out_path: &PathBuf, language: Language) -> anyhow::Result<()> {

    let out_path_str = out_path.to_str().ok_or_else(|| anyhow::anyhow!("Invalid output path"))?;

    let content = std::fs::read_to_string(in_path)?;
    let input_bill: InputBill = match in_path.extension().and_then(|s| s.to_str()) {
        Some("toml") => toml::from_str(&content)?,
        Some("json") => serde_json::from_str(&content)?,
        _ => anyhow::bail!("Use .toml or .json"),
    };

    let bill_data: BillData = input_bill.try_into()?;

    create_pdf(out_path_str, language, &bill_data)?;
    Ok(())
}

fn swiss_theme() -> Theme {
    Theme::custom(
        "Swiss Style",
        Palette {
            background: Color::WHITE,
            text: Color::WHITE,
            primary: Color::BLACK,
            success: Color::from_rgb(0.0, 0.5, 0.0),
            warning: Color::from_rgb(0.5, 0.5, 0.0),
            danger: SWISS_RED,
        }
    )
}

fn primary_button(_theme: &iced::Theme, status: button::Status) -> button::Style {
    let base_style = button::Style {
        background: Some(Background::Color(SWISS_RED)),
        text_color: Color::WHITE,
        border: Border {
            radius: 10.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        snap: false,
    };

    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(SWISS_RED_HOVER)),
            ..base_style
        },
        button::Status::Pressed => button::Style {
            // Even darker or slightly shifted for the "click"
            background: Some(Background::Color(Color::from_rgb(0.6, 0.05, 0.05))),
            ..base_style
        },
        _ => base_style,
    }
}

fn swiss_picklist(_theme: &iced::Theme, status: pick_list::Status) -> pick_list::Style {
    let base_style = pick_list::Style {
        background: Background::Color(SWISS_RED), // Comma, not semicolon
        text_color: Color::WHITE,
        placeholder_color: Color::from_rgb(0.9, 0.9, 0.9),
        handle_color: Color::WHITE,
        border: Default::default(),
    };

    match status {
        pick_list::Status::Hovered => pick_list::Style {
            background: Background::Color(SWISS_RED_HOVER),
            text_color: Color::WHITE,
            ..base_style
        },
        pick_list::Status::Active => pick_list::Style {
            background: Background::Color(SWISS_RED_HOVER),
            text_color: Color::WHITE,
            ..base_style
        },
        pick_list::Status::Opened { is_hovered } =>
            if is_hovered {
                pick_list::Style {
                    background: Background::Color(SWISS_RED),
                    text_color: Color::WHITE,
                    ..base_style
                }
            } else {
                pick_list::Style {
                    text_color: Color::WHITE,
                    ..base_style
                }
            },
    }
}

fn swiss_menu(_theme: &iced::Theme) -> iced::widget::overlay::menu::Style {

    let swiss_border = Border {
        color: Color::from_rgb(0.4, 0.4, 0.4),
        width: 1.0,
        ..Default::default()
    };

    iced::widget::overlay::menu::Style {
        text_color: Color::BLACK,
        background: Background::Color(Color::WHITE),
        selected_text_color: Color::WHITE,
        selected_background: Background::Color(SWISS_RED),
        border: swiss_border,
        shadow: Default::default(),
    }
}
