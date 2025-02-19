#![windows_subsystem = "windows"]
mod canbus;

use canbus::CanApp;
use eframe::egui;
use flume::{unbounded, Receiver, Sender};

#[derive(Clone)]
struct BaudRateOption {
    name: &'static str,
    timing0: u8,
    timing1: u8,
}

struct MyApp {
    can_app: CanApp,
    dev_type: u32,
    dev_index: u32,
    can_channel: u32,
    log: Vec<String>,
    received_data: Vec<String>,
    log_tx: Sender<String>,
    log_rx: Receiver<String>,
    data_tx: Sender<String>,
    data_rx: Receiver<String>,
    baud_options: Vec<BaudRateOption>,
    selected_baud: usize,
    device_open: bool,
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
        let (log_tx, log_rx) = unbounded();
        let (data_tx, data_rx) = unbounded();

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
            selected_baud: 8, // Default 250 Kbps
            device_open: false,
        }
    }
}

impl MyApp {
    fn get_last_lines(texts: &Vec<String>, n: usize) -> String {
        let start = if texts.len() > n { texts.len() - n } else { 0 };
        texts[start..].join("\n")
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
            ui.horizontal(|ui| {
                if ui.button("打開裝置").clicked() {
                    self.device_open = self.can_app.open_device(
                        self.dev_type,
                        self.dev_index,
                        self.can_channel,
                        self.log_tx.clone(),
                    );
                }

                if ui.button("開始接收").clicked() {
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
                        self.log.push("開始接收訊息".to_string());
                    }
                }
                if ui.button("停止接收").clicked() {
                    self.can_app.stop_receiving();
                    self.log.push("停止接收".to_string());
                }
                if ui.button("關閉裝置").clicked() {
                    self.can_app
                        .close_device(self.dev_type, self.dev_index, self.log_tx.clone());
                    self.device_open = false;
                }
            });

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("裝置類型:");
                ui.add(egui::DragValue::new(&mut self.dev_type));
                ui.label("裝置索引:");
                ui.add(egui::DragValue::new(&mut self.dev_index));
                ui.label("CAN 通道:");
                ui.add(egui::DragValue::new(&mut self.can_channel));
                ui.label("CAN波特率:");
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

                if ui.button("讀取板卡資訊").clicked() {
                    self.can_app.read_board_info(
                        self.dev_type,
                        self.dev_index,
                        self.log_tx.clone(),
                    );
                }
            });
            ui.separator();

            let rec_text = MyApp::get_last_lines(&self.received_data, 8);
            let log_text = MyApp::get_last_lines(&self.log, 8);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label("接收的資料:");
                    ui.label(rec_text);
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.label("日誌:");
                    ui.label(log_text);
                });
            });
        });

        ctx.request_repaint();
    }
}
fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "CAN Bus 控制 App",
        native_options,
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
    .expect("無法啟動應用程式");
}
