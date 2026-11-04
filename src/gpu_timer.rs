//! GPU time of each presented frame, for the on-TV perf log: GL_EXT_disjoint_timer_query on
//! GLES, core timer queries on desktop GL. A frame's result is read frames later, once the GPU
//! has finished it; the CPU never waits for one, and a frame with no free query goes untimed.
use std::collections::VecDeque;
use std::ffi::{CStr, c_void};

const TIME_ELAPSED: u32 = 0x88BF;
const QUERY_RESULT: u32 = 0x8866;
const QUERY_RESULT_AVAILABLE: u32 = 0x8867;
const GPU_DISJOINT: u32 = 0x8FBB;
/// Frames that may wait for the GPU at once; a vsync-paced GPU is at most a few behind.
const DEPTH: usize = 4;

/// Queries free for the next frame, those waiting for the GPU (oldest first), and the one
/// timing the current frame.
#[derive(Default)]
struct Ring {
    free: Vec<u32>,
    pending: VecDeque<u32>,
    open: Option<u32>,
}

impl Ring {
    fn new(ids: Vec<u32>) -> Self {
        Self {
            free: ids,
            ..Self::default()
        }
    }

    /// The query to time this frame with, unless every one still waits for the GPU.
    fn begin(&mut self) -> Option<u32> {
        if self.open.is_some() {
            return None;
        }
        self.open = self.free.pop();
        self.open
    }

    /// Closes this frame's query; it waits for the GPU from here.
    fn end(&mut self) -> Option<u32> {
        let id = self.open.take()?;
        self.pending.push_back(id);
        Some(id)
    }

    /// Milliseconds of the queries the GPU has finished, in the order their frames were drawn.
    /// A disjoint GPU clock (frequency change, power state) voids what was read.
    fn collect(
        &mut self,
        mut available: impl FnMut(u32) -> bool,
        mut nanoseconds: impl FnMut(u32) -> u64,
        disjoint: bool,
    ) -> Vec<f64> {
        let mut times = Vec::new();
        while let Some(&id) = self.pending.front()
            && available(id)
        {
            self.pending.pop_front();
            times.push(nanoseconds(id) as f64 / 1_000_000.);
            self.free.push(id);
        }
        if disjoint {
            times.clear();
        }
        times
    }
}

type Gen = unsafe extern "system" fn(i32, *mut u32);
type Begin = unsafe extern "system" fn(u32, u32);
type End = unsafe extern "system" fn(u32);
type GetU32 = unsafe extern "system" fn(u32, u32, *mut u32);
type GetU64 = unsafe extern "system" fn(u32, u32, *mut u64);

pub struct GpuTimer {
    ring: Ring,
    ids: Vec<u32>,
    gles: bool,
    delete: Gen,
    begin: Begin,
    end: End,
    get_u32: GetU32,
    get_u64: GetU64,
}

impl GpuTimer {
    /// For the current context. `address` resolves a GL entry point, as SDL's does.
    pub fn load(gles: bool, address: impl Fn(&str) -> *const c_void) -> Result<Self, String> {
        if gles {
            // SAFETY: a current context; GetString answers a static NUL-terminated string.
            let extensions = unsafe { gl::GetString(gl::EXTENSIONS) };
            let advertised = !extensions.is_null()
                // SAFETY: non-null, NUL-terminated, owned by the GL implementation.
                && unsafe { CStr::from_ptr(extensions.cast()) }
                    .to_string_lossy()
                    .split_whitespace()
                    .any(|name| name == "GL_EXT_disjoint_timer_query");
            if !advertised {
                return Err("GL_EXT_disjoint_timer_query is not advertised".into());
            }
        } else {
            let (mut major, mut minor) = (0, 0);
            // SAFETY: a current context of GL 3.0 or later answers both.
            unsafe {
                gl::GetIntegerv(gl::MAJOR_VERSION, &mut major);
                gl::GetIntegerv(gl::MINOR_VERSION, &mut minor);
            }
            if (major, minor) < (3, 3) {
                return Err(format!("GL {major}.{minor} has no timer queries"));
            }
        }
        let suffix = if gles { "EXT" } else { "" };
        let entry = |name: &str| {
            let pointer = address(&format!("gl{name}{suffix}"));
            if pointer.is_null() {
                Err(format!("gl{name}{suffix} is missing"))
            } else {
                Ok(pointer)
            }
        };
        // SAFETY: each address is the named GL entry point, with these C signatures.
        let (generate, delete, begin, end, get_u32, get_u64) = unsafe {
            (
                std::mem::transmute::<*const c_void, Gen>(entry("GenQueries")?),
                std::mem::transmute::<*const c_void, Gen>(entry("DeleteQueries")?),
                std::mem::transmute::<*const c_void, Begin>(entry("BeginQuery")?),
                std::mem::transmute::<*const c_void, End>(entry("EndQuery")?),
                std::mem::transmute::<*const c_void, GetU32>(entry("GetQueryObjectuiv")?),
                std::mem::transmute::<*const c_void, GetU64>(entry("GetQueryObjectui64v")?),
            )
        };
        let mut ids = vec![0; DEPTH];
        // SAFETY: room for DEPTH names.
        unsafe { generate(DEPTH as i32, ids.as_mut_ptr()) };
        if ids.contains(&0) {
            return Err("no query objects".into());
        }
        Ok(Self {
            ring: Ring::new(ids.clone()),
            ids,
            gles,
            delete,
            begin,
            end,
            get_u32,
            get_u64,
        })
    }

    /// Starts timing a frame; call after its damage region is set, before its first draw.
    pub fn begin(&mut self) {
        if let Some(id) = self.ring.begin() {
            // SAFETY: a query name this timer generated, not active.
            unsafe { (self.begin)(TIME_ELAPSED, id) };
        }
    }

    /// Stops timing the frame; call after its last draw, before the swap.
    pub fn end(&mut self) {
        if self.ring.end().is_some() {
            // SAFETY: the TIME_ELAPSED query begun for this frame is active.
            unsafe { (self.end)(TIME_ELAPSED) };
        }
    }

    /// GPU milliseconds of the frames finished since the last call, oldest first.
    pub fn collect(&mut self) -> Vec<f64> {
        let (get_u32, get_u64) = (self.get_u32, self.get_u64);
        let times = self.ring.collect(
            |id| {
                let mut ready = 0;
                // SAFETY: an ended query of this timer.
                unsafe { get_u32(id, QUERY_RESULT_AVAILABLE, &mut ready) };
                ready != 0
            },
            |id| {
                let mut nanoseconds = 0;
                // SAFETY: an ended query whose result is available.
                unsafe { get_u64(id, QUERY_RESULT, &mut nanoseconds) };
                nanoseconds
            },
            self.gles && {
                let mut disjoint = 0;
                // SAFETY: GPU_DISJOINT_EXT is a query of the extension load() found.
                unsafe { gl::GetIntegerv(GPU_DISJOINT, &mut disjoint) };
                disjoint != 0
            },
        );
        times
    }
}

impl Drop for GpuTimer {
    fn drop(&mut self) {
        // SAFETY: names this timer generated; deleting an active query ends it.
        unsafe { (self.delete)(self.ids.len() as i32, self.ids.as_ptr().cast_mut()) };
    }
}

#[cfg(test)]
#[path = "gpu-timer-tests.rs"]
mod tests;
