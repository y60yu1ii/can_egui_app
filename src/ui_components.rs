use crate::canbus::CanApp;
use eframe::egui::{self, ScrollArea};
use egui::{Align, Color32, TextStyle};
use flume::{Receiver, Sender};

#[derive(Clone)]
pub struct BaudRateOption {
    pub name: &'static str,
    pub timing0: u8,
    pub timing1: u8,
}

pub struct MyApp {
    pub can_app: CanApp,
    pub dev_type: u32,
    pub dev_index: u32,
    pub can_channel: u32,
    pub log: Vec<String>,
    pub received_data: Vec<String>,
    pub log_tx: Sender<String>,
    pub log_rx: Receiver<String>,
    pub data_tx: Sender<String>,
    pub data_rx: Receiver<String>,
    pub baud_options: Vec<BaudRateOption>,
    pub selected_baud: usize,
    pub device_open: bool,
    pub receiving: bool,
}

impl Default for MyApp {
    fn default() -> Self {
        let baud_options = vec![
            BaudRateOption {
                name: "10 Kbps",
                timing0: 0x31,
                timing1: 0x1C,
            },
            BaudRateOption {
                name: "20 Kbps",
                timing0: 0x18,
                timing1: 0x1C,
            },
            BaudRateOption {
                name: "40 Kbps",
                timing0: 0x87,
                timing1: 0xFF,
            },
            BaudRateOption {
                name: "50 Kbps",
                timing0: 0x09,
                timing1: 0x1C,
            },
            BaudRateOption {
                name: "80 Kbps",
                timing0: 0x83,
                timing1: 0xFF,
            },
            BaudRateOption {
                name: "100 Kbps",
                timing0: 0x04,
                timing1: 0x1C,
            },
            BaudRateOption {
                name: "125 Kbps",
                timing0: 0x03,
                timing1: 0x1C,
            },
            BaudRateOption {
                name: "200 Kbps",
                timing0: 0x81,
                timing1: 0xFA,
            },
            BaudRateOption {
                name: "250 Kbps",
                timing0: 0x01,
                timing1: 0x1C,
            }, // 預設
            BaudRateOption {
                name: "500 Kbps",
                timing0: 0x00,
                timing1: 0x1C,
            },
            BaudRateOption {
                name: "1000 Kbps",
                timing0: 0x00,
                timing1: 0x14,
            },
        ];

        let (log_tx, log_rx) = flume::unbounded();
        let (data_tx, data_rx) = flume::unbounded();

        Self {
            can_app: CanApp::new(),
            dev_type: 4,
            dev_index: 0,
            can_channel: 0,
            log: Vec::new(),
            received_data: Vec::new(),
            log_tx,
            log_rx,
            data_tx,
            data_rx,
            baud_options,
            selected_baud: 8,
            device_open: false,
            receiving: false,
        }
    }
}

impl MyApp {
    // fn get_last_lines(texts: &Vec<String>, n: usize) -> String {
    //     let start = if texts.len() > n { texts.len() - n } else { 0 };
    //     texts[start..].join("\n")
    // }

    pub fn draw_ui(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.log_rx.try_recv() {
            self.log.push(msg);
            if self.log.len() > 100 {
                self.log.drain(0..(self.log.len() - 100));
            }
        }

        while let Ok(msg) = self.data_rx.try_recv() {
            self.received_data.push(msg);
            if self.received_data.len() > 100 {
                self.received_data
                    .drain(0..(self.received_data.len() - 100));
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut fonts = egui::FontDefinitions::default();
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "Microsoft JhengHei".to_owned());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "Microsoft JhengHei".to_owned());

            fonts.font_data.insert(
                "Microsoft JhengHei".to_owned(),
                egui::FontData::from_static(include_bytes!("C:/Windows/Fonts/msjh.ttc")).into(),
            );

            ctx.set_fonts(fonts);

            ui.horizontal(|ui| {
                // 設備狀態燈
                let device_color = if self.device_open {
                    Color32::GREEN
                } else {
                    Color32::RED
                };

                ui.allocate_ui_with_layout(
                    egui::Vec2::new(16.0, 16.0),
                    egui::Layout::left_to_right(Align::Center),
                    |ui| {
                        let (_, rect) = ui.allocate_space(egui::Vec2::new(12.0, 12.0)); // 解構 tuple，取 rect
                        ui.painter().circle_filled(rect.center(), 6.0, device_color);
                    },
                );
                ui.label("CAN");

                let recv_color = if self.receiving {
                    Color32::GREEN
                } else {
                    Color32::RED
                };

                ui.allocate_ui_with_layout(
                    egui::Vec2::new(16.0, 16.0),
                    egui::Layout::left_to_right(Align::Center),
                    |ui| {
                        let (_, rect) = ui.allocate_space(egui::Vec2::new(12.0, 12.0)); // 解構 tuple，取 rect
                        ui.painter().circle_filled(rect.center(), 6.0, recv_color);
                    },
                );
                ui.label("Data");
                if ui.button("打開").clicked() {
                    self.device_open = self.can_app.open_device(
                        self.dev_type,
                        self.dev_index,
                        self.can_channel,
                        self.log_tx.clone(),
                    );
                }

                if ui.button("接收").clicked() {
                    if !self.device_open {
                        self.log.push("錯誤：裝置尚未打開".to_string());
                    } else {
                        self.can_app.start_receiving(
                            self.dev_type,
                            self.dev_index,
                            self.can_channel,
                            self.log_tx.clone(),
                            self.data_tx.clone(),
                        );
                        self.receiving = true;
                        self.log.push("開始接收訊息".to_string());
                    }
                }
                if ui.button("停止").clicked() {
                    self.can_app.stop_receiving();
                    self.log.push("停止接收".to_string());
                    self.receiving = false;
                }
                if ui.button("關閉").clicked() {
                    self.can_app
                        .close_device(self.dev_type, self.dev_index, self.log_tx.clone());
                    self.device_open = false;
                }
                if ui.button("板卡資訊").clicked() {
                    self.can_app.read_board_info(
                        self.dev_type,
                        self.dev_index,
                        self.log_tx.clone(),
                    );
                }
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Type:");
                ui.add(egui::DragValue::new(&mut self.dev_type));
                ui.label("Index:");
                ui.add(egui::DragValue::new(&mut self.dev_index));
                ui.label("Channel:");
                ui.add(egui::DragValue::new(&mut self.can_channel));
                ui.label("Baud:");
                egui::ComboBox::from_label("")
                    .selected_text(self.baud_options[self.selected_baud].name)
                    .show_ui(ui, |ui| {
                        for (i, option) in self.baud_options.iter().enumerate() {
                            ui.selectable_value(&mut self.selected_baud, i, option.name);
                        }
                    });
                if ui.button("重設波特率").clicked() {
                    let option = &self.baud_options[self.selected_baud];
                    self.can_app.reconnect_device(
                        self.dev_type,
                        self.dev_index,
                        self.can_channel,
                        option.timing0,
                        option.timing1,
                        self.log_tx.clone(),
                    );
                }
            });
            ui.separator();

            // let rec_text = MyApp::get_last_lines(&self.received_data, 8);
            // let log_text = MyApp::get_last_lines(&self.log, 8);

            // ui.horizontal(|ui| {
            //     ui.vertical(|ui| {
            //         ui.label("接收的資料:");
            //         ui.label(rec_text);
            //     });
            //     ui.separator();
            //     ui.vertical(|ui| {
            //         ui.label("日誌:");
            //         ui.label(log_text);
            //     });
            // });

            ui.vertical(|ui| {
                ui.label("日誌:");

                let row_height = ui.text_style_height(&TextStyle::Body);
                let visible_lines = 2;
                let scroll_height = row_height * visible_lines as f32;

                ScrollArea::vertical()
                    .id_salt("log_scroll")
                    .max_height(scroll_height)
                    .min_scrolled_height(scroll_height)
                    .auto_shrink([true; 2])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.allocate_space(egui::vec2(ui.available_width(), scroll_height));
                        for line in self.log.iter().rev().take(10).rev() {
                            ui.label(line);
                        }
                    });
                ui.separator();
                ui.label("接收的資料:");

                let row_height = ui.text_style_height(&TextStyle::Body);
                let visible_lines = 10;
                let scroll_height = row_height * visible_lines as f32;

                ScrollArea::vertical()
                    .id_salt("received_data_scroll")
                    .max_height(scroll_height)
                    .min_scrolled_height(scroll_height)
                    .auto_shrink([true; 2])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        ui.allocate_space(egui::vec2(ui.available_width(), scroll_height));
                        for line in self.received_data.iter().rev().take(1000).rev() {
                            ui.label(line);
                        }
                    });
            });
        });

        ctx.request_repaint();
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.draw_ui(ctx);
    }
}
