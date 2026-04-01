// egui backend — uses eframe, which uses winit internally.
// winit::dpi::LogicalSize is used here directly to define the initial window dimensions.

use eframe::egui;
use winit::dpi::LogicalSize;

use super::ContactCard;

struct VenturiCardsEgui {
    card: ContactCard,
    contact_index: usize,
    total_contacts: usize,
}

impl VenturiCardsEgui {
    fn new(card: ContactCard) -> Self {
        Self {
            card,
            contact_index: 1,
            total_contacts: 3,
        }
    }
}

impl eframe::App for VenturiCardsEgui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── Title bar ──────────────────────────────────────────────────
        egui::TopBottomPanel::top("titlebar").show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new("V E N T U R I   C A R D S")
                        .size(20.0)
                        .strong(),
                );
                ui.label(egui::RichText::new("v 1.0").size(12.0));
                ui.add_space(10.0);
            });
        });

        // ── Status / action bar ───────────────────────────────────────
        egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let _ = ui.button("New");
                let _ = ui.button("Edit");
                let _ = ui.button("Share");
                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        let _ = ui.button("Search");
                        ui.label(format!(
                            "Contact {} of {}",
                            self.contact_index, self.total_contacts
                        ));
                    },
                );
            });
            ui.add_space(6.0);
        });

        // ── Contact card ──────────────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::same(20))
                .show(ui, |ui| {
                    egui::Grid::new("card_grid")
                        .num_columns(2)
                        .spacing([24.0, 10.0])
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Name").strong());
                            ui.label(&self.card.name);
                            ui.end_row();

                            ui.label(egui::RichText::new("Role").strong());
                            ui.label(&self.card.role);
                            ui.end_row();

                            ui.label(egui::RichText::new("Email").strong());
                            ui.label(&self.card.email);
                            ui.end_row();

                            ui.label(egui::RichText::new("Tags").strong());
                            ui.label(&self.card.tags);
                            ui.end_row();
                        });
                });
        });
    }
}

pub fn run(card: ContactCard) -> crate::error::Result<()> {
    // Use winit's LogicalSize to define window dimensions.
    let win: LogicalSize<f32> = LogicalSize::new(500.0, 340.0);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("VenturiCards")
            .with_inner_size([win.width, win.height])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "VenturiCards",
        options,
        Box::new(|_cc| Ok(Box::new(VenturiCardsEgui::new(card)))),
    )
    .map_err(|e| crate::error::VenturiError::Gui(e.to_string()))
}
