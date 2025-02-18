use flume::Sender;
use libloading::Library;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

#[repr(C)]
#[derive(Debug, Default)]
pub struct VciCanObj {
    pub id: u32,
    pub time_stamp: u32,
    pub time_flag: u8,
    pub send_type: u8,
    pub remote_flag: u8,
    pub extern_flag: u8,
    pub data_len: u8,
    pub data: [u8; 8],
    pub reserved: [u8; 3],
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct VciInitConfig {
    pub acc_code: u32,
    pub acc_mask: u32,
    pub reserved: u32,
    pub filter: u8,
    pub timing0: u8,
    pub timing1: u8,
    pub mode: u8,
}

#[repr(C)]
#[derive(Debug)]
pub struct VciBoardInfo {
    pub hw_version: u16,
    pub fw_version: u16,
    pub dr_version: u16,
    pub in_version: u16,
    pub irq_num: u16,
    pub can_num: u8,
    pub str_serial_num: [u8; 20],
    pub str_hw_type: [u8; 40],
    pub reserved: [u16; 4],
}

impl Default for VciBoardInfo {
    fn default() -> Self {
        Self {
            hw_version: 0,
            fw_version: 0,
            dr_version: 0,
            in_version: 0,
            irq_num: 0,
            can_num: 0,
            str_serial_num: [0; 20],
            str_hw_type: [0; 40],
            reserved: [0; 4],
        }
    }
}

#[derive(Debug)]
pub struct DeviceInfo {
    // pub index: i32,
    pub serial_number: String,
    pub firmware_version: u16,
}

pub struct CanLibrary {
    _lib: Arc<Library>,
    pub vci_open_device: unsafe extern "stdcall" fn(u32, u32, u32) -> i32,
    pub vci_close_device: unsafe extern "stdcall" fn(u32, u32) -> i32,
    pub vci_init_can: unsafe extern "stdcall" fn(u32, u32, u32, *const VciInitConfig) -> i32,
    pub vci_start_can: unsafe extern "stdcall" fn(u32, u32, u32) -> i32,
    // pub vci_transmit: unsafe extern "stdcall" fn(u32, u32, u32, *const VciCanObj, u32) -> i32,
    pub vci_receive: unsafe extern "stdcall" fn(u32, u32, u32, *mut VciCanObj, u32, i32) -> i32,
    // pub vci_find_usb_device2: unsafe extern "stdcall" fn(*mut VciBoardInfo) -> i32,
    pub vci_read_board_info: unsafe extern "stdcall" fn(u32, u32, *mut VciBoardInfo) -> i32,
}

impl CanLibrary {
    /// 載入 DLL 並取得所需的函數指標
    pub fn new(dll_name: &str) -> Arc<Self> {
        let lib = Arc::new(unsafe { Library::new(dll_name) }.expect("DLL load failed"));
        unsafe {
            Arc::new(Self {
                _lib: lib.clone(),
                vci_open_device: *lib
                    .get(b"VCI_OpenDevice")
                    .expect("Failed to get VCI_OpenDevice"),
                vci_close_device: *lib
                    .get(b"VCI_CloseDevice")
                    .expect("Failed to get VCI_CloseDevice"),
                vci_init_can: *lib.get(b"VCI_InitCAN").expect("Failed to get VCI_InitCAN"),
                vci_start_can: *lib
                    .get(b"VCI_StartCAN")
                    .expect("Failed to get VCI_StartCAN"),
                // vci_transmit: *lib
                //     .get(b"VCI_Transmit")
                //     .expect("Failed to get VCI_Transmit"),
                vci_receive: *lib.get(b"VCI_Receive").expect("Failed to get VCI_Receive"),
                // vci_find_usb_device2: *lib
                //     .get(b"VCI_FindUsbDevice2")
                //     .expect("Failed to get VCI_FindUsbDevice2"),
                vci_read_board_info: *lib
                    .get(b"VCI_ReadBoardInfo")
                    .expect("Failed to get VCI_ReadBoardInfo"),
            })
        }
    }
}

pub struct CanApp {
    pub can_lib: Arc<CanLibrary>,
    pub receiving: Arc<AtomicBool>,
}

impl CanApp {
    pub fn new() -> Self {
        let can_lib = CanLibrary::new("ControlCAN.dll");
        Self {
            can_lib,
            receiving: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn open_device(&self, dev_type: u32, dev_index: u32) -> bool {
        unsafe { (self.can_lib.vci_open_device)(dev_type, dev_index, 0) == 1 }
    }

    pub fn close_device(&self, dev_type: u32, dev_index: u32) {
        unsafe {
            (self.can_lib.vci_close_device)(dev_type, dev_index);
        }
    }

    // pub fn transmit_data(&self, dev_type: u32, dev_index: u32, can_channel: u32, data: u8) {
    //     let can_obj = VciCanObj {
    //         id: 0x1,
    //         data_len: 1,
    //         data: [data, 0, 0, 0, 0, 0, 0, 0],
    //         ..Default::default()
    //     };
    //     unsafe {
    //         (self.can_lib.vci_transmit)(dev_type, dev_index, can_channel, &can_obj, 1);
    //     }
    // }

    pub fn start_receiving(
        &self,
        dev_type: u32,
        dev_index: u32,
        can_channel: u32,
        sender: Sender<String>,
    ) {
        let receiving_flag = Arc::clone(&self.receiving);
        receiving_flag.store(true, Ordering::SeqCst);
        let can_lib = Arc::clone(&self.can_lib);
        std::thread::spawn(move || {
            while receiving_flag.load(Ordering::SeqCst) {
                let mut can_obj = VciCanObj::default();
                let received_frames = unsafe {
                    (can_lib.vci_receive)(dev_type, dev_index, can_channel, &mut can_obj, 1, 500)
                };
                if received_frames == 995 {
                    // 忽略 995 錯誤，直接跳過本次迴圈
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                }
                if received_frames > 0 {
                    let data = &can_obj.data[..(can_obj.data_len as usize)];
                    let msg = format!("ID=0x{:X}, Data={:?}", can_obj.id, data);
                    let _ = sender.send(msg);
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        });
    }

    pub fn stop_receiving(&self) {
        self.receiving.store(false, Ordering::SeqCst);
    }

    /// 讀取板卡資訊並回傳 DeviceInfo
    pub fn read_board_info(&self, dev_type: u32, dev_index: u32) -> Result<DeviceInfo, String> {
        let mut board_info = VciBoardInfo::default();
        unsafe {
            let status = (self.can_lib.vci_read_board_info)(dev_type, dev_index, &mut board_info);
            if status != 1 {
                return Err("Failed to read board info".to_string());
            }
        }
        Ok(DeviceInfo {
            serial_number: String::from_utf8_lossy(&board_info.str_serial_num)
                .trim_matches('\0')
                .to_string(),
            firmware_version: board_info.fw_version,
        })
    }

    // /// 設定波特率（初始化 CAN），成功回傳 Ok(())
    // pub fn set_baud_rate(
    //     &self,
    //     dev_type: u32,
    //     dev_index: u32,
    //     can_channel: u32,
    //     timing0: u8,
    //     timing1: u8,
    // ) -> Result<(), String> {
    //     let config = VciInitConfig {
    //         acc_code: 0,
    //         acc_mask: 0xFFFFFFFF,
    //         reserved: 0,
    //         filter: 1,
    //         timing0,
    //         timing1,
    //         mode: 0,
    //     };
    //     unsafe {
    //         if (self.can_lib.vci_init_can)(dev_type, dev_index, can_channel, &config) != 1 {
    //             return Err("Failed to set baud rate".to_string());
    //         }
    //     }
    //     Ok(())
    // }

    pub fn reconnect_device(
        &mut self,
        dev_type: u32,
        dev_index: u32,
        can1: u32,
        can2: u32,
        timing0: u8,
        timing1: u8,
    ) -> Result<(), String> {
        unsafe {
            (self.can_lib.vci_close_device)(dev_type, dev_index);
            if (self.can_lib.vci_open_device)(dev_type, dev_index, 0) != 1 {
                return Err("Failed to open device".into());
            }
        }
        let config = VciInitConfig {
            acc_code: 0,
            acc_mask: 0xFFFFFFFF,
            reserved: 0,
            filter: 1,
            timing0,
            timing1,
            mode: 0,
        };
        unsafe {
            if (self.can_lib.vci_init_can)(dev_type, dev_index, can1, &config) != 1 {
                return Err("Failed to initialize CAN1 with new baud".into());
            }
            if (self.can_lib.vci_init_can)(dev_type, dev_index, can2, &config) != 1 {
                return Err("Failed to initialize CAN2 with new baud".into());
            }
            if (self.can_lib.vci_start_can)(dev_type, dev_index, can1) != 1 {
                return Err("Failed to start CAN1 after reconnect".into());
            }
            if (self.can_lib.vci_start_can)(dev_type, dev_index, can2) != 1 {
                return Err("Failed to start CAN2 after reconnect".into());
            }
        }
        Ok(())
    }
}
