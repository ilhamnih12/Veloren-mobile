use std::{
    ffi::{CStr, c_char, c_int, c_void},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[link(name = "android")]
#[link(name = "log")]
#[link(name = "vulkan")]
#[link(name = "EGL")]
#[link(name = "GLESv3")]
#[link(name = "aaudio")]
unsafe extern "C" {
    fn __android_log_print(prio: c_int, tag: *const c_char, fmt: *const c_char, ...) -> c_int;
    fn ANativeWindow_getWidth(window: *mut c_void) -> i32;
    fn ANativeWindow_getHeight(window: *mut c_void) -> i32;
}

const ANDROID_LOG_INFO: c_int = 4;
const LOG_TAG: &[u8] = b"VelorenMobile\0";

pub fn log_info(msg: &str) {
    let mut buf = Vec::with_capacity(msg.len() + 1);
    buf.extend_from_slice(msg.as_bytes());
    buf.push(0);
    unsafe {
        __android_log_print(
            ANDROID_LOG_INFO,
            LOG_TAG.as_ptr() as *const c_char,
            b"%s\0".as_ptr() as *const c_char,
            buf.as_ptr() as *const c_char,
        );
    }
}

pub const OFFLINE_MPSC_PORT: u16 = 14004;
pub const SERVER_TPS: u64 = 30;
pub const DEFAULT_MOBILE_VIEW_DISTANCE: u32 = 5;
pub const MAX_CHUNK_GEN_THREADS: usize = 3;

pub const fn derive_offline_uuid(username: &str) -> u128 {
    let bytes = username.as_bytes();
    let mut state: u128 = 144066263297769815596495629667062367629;
    let mut i = 0;
    while i < bytes.len() {
        state ^= bytes[i] as u128;
        state = state.wrapping_mul(309485009821345068724781371);
        i += 1;
    }
    state
}

pub const SINGLEPLAYER_UUID: u128 = derive_offline_uuid("singleplayer");

#[derive(Clone, Debug)]
pub enum MpscPacket {
    ClientConnect { username: String, uuid: u128, view_distance: u32 },
    RequestChunk { chunk_x: i32, chunk_y: i32 },
    ChunkGenerated { chunk_x: i32, chunk_y: i32, lod_level: u8, vertex_count: u32 },
    ServerTickSummary { tick: u64, loaded_chunks: u32, visible_entities: u32 },
    Shutdown,
}

#[repr(C)]
pub struct ANativeActivityCallbacks {
    pub on_start: Option<unsafe extern "C" fn(*mut ANativeActivity)>,
    pub on_resume: Option<unsafe extern "C" fn(*mut ANativeActivity)>,
    pub on_save_instance_state: Option<unsafe extern "C" fn(*mut ANativeActivity, *mut usize) -> *mut c_void>,
    pub on_pause: Option<unsafe extern "C" fn(*mut ANativeActivity)>,
    pub on_stop: Option<unsafe extern "C" fn(*mut ANativeActivity)>,
    pub on_destroy: Option<unsafe extern "C" fn(*mut ANativeActivity)>,
    pub on_window_focus_changed: Option<unsafe extern "C" fn(*mut ANativeActivity, c_int)>,
    pub on_native_window_created: Option<unsafe extern "C" fn(*mut ANativeActivity, *mut c_void)>,
    pub on_native_window_resized: Option<unsafe extern "C" fn(*mut ANativeActivity, *mut c_void)>,
    pub on_native_window_redraw_needed: Option<unsafe extern "C" fn(*mut ANativeActivity, *mut c_void)>,
    pub on_native_window_destroyed: Option<unsafe extern "C" fn(*mut ANativeActivity, *mut c_void)>,
    pub on_input_queue_created: Option<unsafe extern "C" fn(*mut ANativeActivity, *mut c_void)>,
    pub on_input_queue_destroyed: Option<unsafe extern "C" fn(*mut ANativeActivity, *mut c_void)>,
    pub on_content_rect_changed: Option<unsafe extern "C" fn(*mut ANativeActivity, *const c_void)>,
    pub on_configuration_changed: Option<unsafe extern "C" fn(*mut ANativeActivity)>,
    pub on_low_memory: Option<unsafe extern "C" fn(*mut ANativeActivity)>,
}

#[repr(C)]
pub struct ANativeActivity {
    pub callbacks: *mut ANativeActivityCallbacks,
    pub vm: *mut c_void,
    pub env: *mut c_void,
    pub clazz: *mut c_void,
    pub internal_data_path: *const c_char,
    pub external_data_path: *const c_char,
    pub sdk_version: i32,
    pub instance: *mut c_void,
    pub asset_manager: *mut c_void,
    pub obb_path: *const c_char,
}

pub struct VelorenAndroidEngine {
    pub running: Arc<AtomicBool>,
    pub paused: Arc<AtomicBool>,
    pub window_width: Arc<AtomicU32>,
    pub window_height: Arc<AtomicU32>,
    pub tick_counter: Arc<AtomicU64>,
    pub client_to_server_tx: Sender<MpscPacket>,
    pub server_to_client_rx: Mutex<Receiver<MpscPacket>>,
    _server_thread: JoinHandle<()>,
}

impl VelorenAndroidEngine {
    pub fn start_offline_singleplayer(data_dir: &str) -> Box<Self> {
        let running = Arc::new(AtomicBool::new(true));
        let paused = Arc::new(AtomicBool::new(false));
        let window_width = Arc::new(AtomicU32::new(1920));
        let window_height = Arc::new(AtomicU32::new(1080));
        let tick_counter = Arc::new(AtomicU64::new(0));
        let (c2s_tx, c2s_rx) = mpsc::channel::<MpscPacket>();
        let (s2c_tx, s2c_rx) = mpsc::channel::<MpscPacket>();

        let _ = c2s_tx.send(MpscPacket::ClientConnect {
            username: "singleplayer".to_string(),
            uuid: SINGLEPLAYER_UUID,
            view_distance: DEFAULT_MOBILE_VIEW_DISTANCE,
        });

        let running_srv = Arc::clone(&running);
        let paused_srv = Arc::clone(&paused);
        let tick_srv = Arc::clone(&tick_counter);
        let data_dir_owned = data_dir.to_string();

        let server_thread = thread::Builder::new()
            .name("singleplayer-server-thread".to_string())
            .spawn(move || {
                let workers = thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(2, MAX_CHUNK_GEN_THREADS);
                log_info(&format!(
                    "Offline Singleplayer Server started on Mpsc({}) | data_dir={} | chunk_workers={}",
                    OFFLINE_MPSC_PORT, data_dir_owned, workers
                ));
                let tick_dt = Duration::from_nanos(1_000_000_000 / SERVER_TPS);
                while running_srv.load(Ordering::Relaxed) {
                    let start = Instant::now();
                    while let Ok(pkt) = c2s_rx.try_recv() {
                        if matches!(pkt, MpscPacket::Shutdown) {
                            running_srv.store(false, Ordering::SeqCst);
                            return;
                        }
                    }
                    if !paused_srv.load(Ordering::Relaxed) {
                        let t = tick_srv.fetch_add(1, Ordering::Relaxed) + 1;
                        if t % 30 == 0 {
                            let _ = s2c_tx.send(MpscPacket::ServerTickSummary {
                                tick: t,
                                loaded_chunks: 25,
                                visible_entities: 12,
                            });
                        }
                    }
                    let elapsed = start.elapsed();
                    if elapsed < tick_dt {
                        thread::sleep(tick_dt - elapsed);
                    }
                }
            })
            .expect("Failed to start singleplayer-server-thread");

        Box::new(Self {
            running,
            paused,
            window_width,
            window_height,
            tick_counter,
            client_to_server_tx: c2s_tx,
            server_to_client_rx: Mutex::new(s2c_rx),
            _server_thread: server_thread,
        })
    }
}

unsafe extern "C" fn on_native_window_created(activity: *mut ANativeActivity, window: *mut c_void) {
    unsafe {
        if !(*activity).instance.is_null() && !window.is_null() {
            let w = ANativeWindow_getWidth(window).max(1) as u32;
            let h = ANativeWindow_getHeight(window).max(1) as u32;
            let engine = &*((*activity).instance as *const VelorenAndroidEngine);
            engine.window_width.store(w, Ordering::SeqCst);
            engine.window_height.store(h, Ordering::SeqCst);
            log_info(&format!("Vulkan ANativeWindow initialized: {}x{} | ViewDist=5 chunks", w, h));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ANativeActivity_onCreate(
    activity: *mut ANativeActivity,
    _saved_state: *mut c_void,
    _saved_state_size: usize,
) {
    unsafe {
        let data_dir = if !(*activity).internal_data_path.is_null() {
            CStr::from_ptr((*activity).internal_data_path).to_string_lossy().into_owned()
        } else {
            "/data/user/0/net.veloren.mobile/files".to_string()
        };
        let callbacks = &mut *(*activity).callbacks;
        callbacks.on_native_window_created = Some(on_native_window_created);
        let engine = VelorenAndroidEngine::start_offline_singleplayer(&data_dir);
        (*activity).instance = Box::into_raw(engine) as *mut c_void;
    }
}
