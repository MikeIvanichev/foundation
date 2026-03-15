#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

/// Default jemalloc runtime configuration used when the `jemalloc` feature is
/// enabled on Linux.
pub const DEFAULT_MALLOC_CONF: &str = "prof:true,prof_active:false,lg_prof_sample:19";

#[cfg(target_os = "linux")]
const DEFAULT_MALLOC_CONF_BYTES: &[u8] = b"prof:true,prof_active:false,lg_prof_sample:19\0";

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

// On Linux, keep profiling compiled in but inactive so it can be enabled later
// without imposing meaningful overhead in the default case.
#[cfg(target_os = "linux")]
#[unsafe(export_name = "malloc_conf")]
pub static MALLOC_CONF: &[u8] = DEFAULT_MALLOC_CONF_BYTES;

#[cfg(not(target_os = "linux"))]
pub static MALLOC_CONF: &[u8] = b"";
