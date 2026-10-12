//! Bootstrap mach transport for framed PCM.
//!
//! The HAL plug-in hosts [`SERVICE_NAME`] with `bootstrap_check_in` and
//! receives packets. The userspace daemon looks the service up and sends.
//! When the plug-in is not loaded, `bootstrap_look_up` fails and the daemon
//! surfaces [`MachError::Bootstrap`].

use crate::frame::{decode_packet, encode_packet, PacketHeader, HEADER_LEN, MAX_SAMPLES};
use crate::SERVICE_NAME;
use mach2::kern_return::KERN_SUCCESS;
use mach2::mach_port::mach_port_deallocate;
use mach2::message::{
    mach_msg, mach_msg_header_t, MACH_MSGH_BITS, MACH_MSG_SUCCESS, MACH_MSG_TIMEOUT_NONE,
    MACH_MSG_TYPE_COPY_SEND, MACH_RCV_MSG, MACH_RCV_TIMEOUT, MACH_SEND_MSG,
};
use mach2::port::{mach_port_t, MACH_PORT_NULL};
use mach2::task::{task_get_special_port, TASK_BOOTSTRAP_PORT};
use mach2::traps::mach_task_self;
use mach2::vm_types::integer_t;
use std::ffi::CString;
use std::mem::{size_of, MaybeUninit};
use std::os::raw::c_char;
use std::ptr;
use thiserror::Error;

const MSG_PCM: i32 = 200;
const MAX_INLINE: usize = HEADER_LEN + MAX_SAMPLES * 4;

#[repr(C)]
struct PcmMsg {
    header: mach_msg_header_t,
    data_size: u32,
    data: [u8; MAX_INLINE],
}

/// Errors from bootstrap registration or mach messaging.
#[derive(Debug, Error)]
pub enum MachError {
    /// `bootstrap_check_in` / `bootstrap_look_up` failed.
    #[error("bootstrap error: {0}")]
    Bootstrap(i32),
    /// Port allocation failed.
    #[error("mach port error: {0}")]
    Port(i32),
    /// `mach_msg` send/receive failed.
    #[error("mach_msg error: {0}")]
    Message(i32),
    /// Framing failure after a successful receive.
    #[error(transparent)]
    Frame(#[from] crate::frame::FrameError),
    /// Service name was not a valid `CString`.
    #[error("invalid service name")]
    BadName,
}

extern "C" {
    fn bootstrap_check_in(
        bp: mach_port_t,
        service_name: *const c_char,
        sp: *mut mach_port_t,
    ) -> integer_t;
    fn bootstrap_look_up(
        bp: mach_port_t,
        service_name: *const c_char,
        sp: *mut mach_port_t,
    ) -> integer_t;
}

fn bootstrap_port() -> Result<mach_port_t, MachError> {
    let mut port: mach_port_t = MACH_PORT_NULL;
    // SAFETY: out-param receives the task bootstrap port on success.
    let kr = unsafe { task_get_special_port(mach_task_self(), TASK_BOOTSTRAP_PORT, &mut port) };
    if kr != KERN_SUCCESS {
        return Err(MachError::Port(kr));
    }
    Ok(port)
}

fn empty_header() -> mach_msg_header_t {
    // SAFETY: zeroed header is a valid starting point before fields are set.
    unsafe { MaybeUninit::<mach_msg_header_t>::zeroed().assume_init() }
}

/// Plug-in side: hosts the bootstrap service and receives PCM.
pub struct PluginServer {
    service: mach_port_t,
}

impl PluginServer {
    /// Register [`SERVICE_NAME`] via `bootstrap_check_in`.
    ///
    /// # Errors
    ///
    /// Returns [`MachError`] when bootstrap registration fails.
    pub fn bind() -> Result<Self, MachError> {
        Self::bind_named(SERVICE_NAME)
    }

    /// Register `name` via `bootstrap_check_in`.
    ///
    /// # Errors
    ///
    /// Returns [`MachError`] when the name is invalid or bootstrap fails.
    pub fn bind_named(name: &str) -> Result<Self, MachError> {
        let bp = bootstrap_port()?;
        let c_name = CString::new(name).map_err(|_| MachError::BadName)?;
        let mut service = MACH_PORT_NULL;
        // SAFETY: valid C string; `service` is a writable out-param.
        let kr = unsafe { bootstrap_check_in(bp, c_name.as_ptr(), &mut service) };
        if kr != KERN_SUCCESS {
            return Err(MachError::Bootstrap(kr));
        }
        Ok(Self { service })
    }

    /// Receive one PCM packet.
    ///
    /// `timeout_ms == u32::MAX` waits forever; any other value is a finite timeout.
    ///
    /// # Errors
    ///
    /// Returns [`MachError`] on timeout, receive failure, or bad framing.
    pub fn recv_packet(&self, timeout_ms: u32) -> Result<(PacketHeader, Vec<f32>), MachError> {
        let mut storage = vec![0_u8; size_of::<PcmMsg>()];
        let (options, timeout) = if timeout_ms == u32::MAX {
            (MACH_RCV_MSG, MACH_MSG_TIMEOUT_NONE)
        } else {
            (MACH_RCV_MSG | MACH_RCV_TIMEOUT, timeout_ms)
        };
        // SAFETY: buffer is large enough for `PcmMsg`.
        let recv_size = u32::try_from(storage.len()).map_err(|_| MachError::BadName)?;
        let kr = unsafe {
            mach_msg(
                storage.as_mut_ptr().cast(),
                options,
                0,
                recv_size,
                self.service,
                timeout,
                MACH_PORT_NULL,
            )
        };
        if kr != MACH_MSG_SUCCESS {
            return Err(MachError::Message(kr));
        }
        // SAFETY: receive succeeded; header bytes are initialized.
        let header = unsafe { ptr::read_unaligned(storage.as_ptr().cast::<mach_msg_header_t>()) };
        if header.msgh_id != MSG_PCM {
            return Err(MachError::Message(header.msgh_id));
        }
        let data_size_offset = size_of::<mach_msg_header_t>();
        let mut size_bytes = [0_u8; 4];
        size_bytes.copy_from_slice(&storage[data_size_offset..data_size_offset + 4]);
        let data_size = u32::from_ne_bytes(size_bytes) as usize;
        let data_offset = data_size_offset + 4;
        let end = data_offset.saturating_add(data_size).min(storage.len());
        Ok(decode_packet(&storage[data_offset..end])?)
    }
}

impl Drop for PluginServer {
    fn drop(&mut self) {
        if self.service != MACH_PORT_NULL {
            // SAFETY: deallocate the service receive right we own.
            unsafe {
                let _ = mach_port_deallocate(mach_task_self(), self.service);
            }
        }
    }
}

/// Daemon side: looks up the plug-in's service and sends framed PCM.
pub struct DaemonClient {
    service: mach_port_t,
}

impl DaemonClient {
    /// Look up [`SERVICE_NAME`].
    ///
    /// # Errors
    ///
    /// Returns [`MachError`] when the plug-in has not registered the service.
    pub fn connect() -> Result<Self, MachError> {
        Self::connect_named(SERVICE_NAME)
    }

    /// Look up `name`.
    ///
    /// # Errors
    ///
    /// Returns [`MachError`] on bootstrap failure.
    pub fn connect_named(name: &str) -> Result<Self, MachError> {
        let bp = bootstrap_port()?;
        let c_name = CString::new(name).map_err(|_| MachError::BadName)?;
        let mut service = MACH_PORT_NULL;
        // SAFETY: valid C string; `service` is a writable out-param.
        let kr = unsafe { bootstrap_look_up(bp, c_name.as_ptr(), &mut service) };
        if kr != KERN_SUCCESS {
            return Err(MachError::Bootstrap(kr));
        }
        Ok(Self { service })
    }

    /// Send one framed PCM packet.
    ///
    /// # Errors
    ///
    /// Returns [`MachError`] when framing or send fails.
    pub fn send_packet(&self, header: PacketHeader, samples: &[f32]) -> Result<(), MachError> {
        let bytes = encode_packet(header, samples)?;
        if bytes.len() > MAX_INLINE {
            return Err(MachError::Frame(crate::frame::FrameError::TooManySamples {
                got: u32::try_from(samples.len()).unwrap_or(u32::MAX),
            }));
        }
        let data_size = u32::try_from(bytes.len()).map_err(|_| MachError::BadName)?;
        let msg_size =
            u32::try_from(size_of::<mach_msg_header_t>() + size_of::<u32>() + bytes.len())
                .map_err(|_| MachError::BadName)?;
        let mut msg = Box::new(PcmMsg {
            header: empty_header(),
            data_size,
            data: [0; MAX_INLINE],
        });
        msg.data[..bytes.len()].copy_from_slice(&bytes);
        msg.header.msgh_bits = MACH_MSGH_BITS(MACH_MSG_TYPE_COPY_SEND, 0);
        msg.header.msgh_size = msg_size;
        msg.header.msgh_remote_port = self.service;
        msg.header.msgh_local_port = MACH_PORT_NULL;
        msg.header.msgh_id = MSG_PCM;
        // SAFETY: `msg` is a well-formed simple mach message.
        let kr = unsafe {
            mach_msg(
                ptr::from_mut(&mut msg.header),
                MACH_SEND_MSG,
                msg.header.msgh_size,
                0,
                MACH_PORT_NULL,
                MACH_MSG_TIMEOUT_NONE,
                MACH_PORT_NULL,
            )
        };
        if kr != MACH_MSG_SUCCESS {
            return Err(MachError::Message(kr));
        }
        Ok(())
    }
}

impl Drop for DaemonClient {
    fn drop(&mut self) {
        if self.service != MACH_PORT_NULL {
            // SAFETY: drop the send right from look_up.
            unsafe {
                let _ = mach_port_deallocate(mach_task_self(), self.service);
            }
        }
    }
}
