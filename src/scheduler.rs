use super::process::{CpuContext, Process, ProcessState};
use crate::println;
use alloc::{boxed::Box, collections::vec_deque::VecDeque};
use core::{arch::global_asm, fmt::Display};

const MAX_PROCESSES: usize = 2;

pub struct Scheduler {
    processes: VecDeque<Process>,
    next_proc_id: u32,
    current_running: Option<Process>,
    previously_running: Option<Process>,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ProcessInfo {
    pid: u32,
    state: ProcessState,
    stack_pointer: usize,
}

impl ProcessInfo {
    pub fn from(process: &Process) -> Self {
        ProcessInfo {
            pid: process.pid,
            state: process.state,
            stack_pointer: process.context.sp,
        }
    }
}
impl Display for ProcessInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "proc with id {}({:?}): {:#x} ",
            self.pid, self.state, self.stack_pointer
        )
    }
}

impl Scheduler {
    pub fn new() -> Self {
        let processes: VecDeque<Process> = VecDeque::new();
        Scheduler {
            processes,
            next_proc_id: 1,
            current_running: None,
            previously_running: None,
        }
    }

    /// TODO: Make this mandatory from within the type system
    pub fn init(&mut self) {
        self.processes.reserve(MAX_PROCESSES);
        self.current_running = Some(Self::create_idle_process());
    }

    pub fn exit_process(&mut self) {
        if self.current_running.is_none() {
            panic!("Exiting a unexisting process")
        }
        self.current_running.as_mut().unwrap().state = ProcessState::Exited;
        self.yield_control();
    }
    pub fn schedule_process(&mut self, entry_point: usize) -> ProcessInfo {
        let new_proc = Process {
            pid: self.next_proc_id,
            state: ProcessState::Runnable,
            ..Default::default()
        };
        println!(
            "Process {}: kernel_stack at {:p}",
            new_proc.pid,
            new_proc.kernel_stack.as_ptr()
        );
        self.next_proc_id += 1;
        self.processes.push_back(new_proc);
        let new_proc = self.processes.back_mut().unwrap();
        Self::init_process(new_proc, entry_point);
        ProcessInfo::from(new_proc)
    }

    fn shedule_idle() {
        panic!("Kernel Idle")
    }

    fn create_idle_process() -> Process {
        let mut idle_process = Process {
            pid: 0,
            state: ProcessState::KernelReserved,
            kernel_stack: Box::new([0; 8192]),
            context: CpuContext::default(),
        };
        let idle_entry = Self::shedule_idle as *const () as usize;
        Self::init_process(&mut idle_process, idle_entry);
        idle_process
    }
    fn init_process(proc: &mut Process, entry_point: usize) {
        unsafe {
            let sp = proc.kernel_stack.as_mut_ptr().add(proc.kernel_stack.len());
            assert!(
                sp as usize % 16 == 0,
                "stack_pointer is not 16-byte aligned"
            );
            proc.context.sp = sp as usize;
            proc.context.ra = entry_point;
        }
    }

    pub extern "C" fn yield_control(&mut self) {
        if self.current_running.is_none() {
            panic!("Cannot yield without having inited the sheduler")
        }

        //TODO: This previously_running thing is a hack to account for a fact
        //we don't yet have an ARC type that can allow use to still use the previous when doing context switch
        if let Some(prev) = self.previously_running.take() {
            if prev.state == ProcessState::Runnable {
                self.processes.push_back(prev);
            }
        }

        self.previously_running = self.current_running.take();
        let proc = self.processes.pop_front();
        self.current_running = match proc {
            None if self.previously_running.as_ref().unwrap().state == ProcessState::Runnable => {
                println!("Nothing in the process-queue to yield to, but previous still runnable");
                self.previously_running.take()
            }
            None => {
                println!("Nothing in the process-queue to yield to, going idle!");
                Some(Self::create_idle_process())
            }
            Some(p) => Some(p),
        };
        println!(
            "Switching from {} to {}",
            self.previously_running.as_ref().unwrap().pid,
            self.current_running.as_ref().unwrap().pid
        );
        Self::switch_context(
            &self.previously_running.as_mut().unwrap().context,
            &self.current_running.as_mut().unwrap().context,
        );
    }

    #[no_mangle]
    extern "C" fn switch_context(prev_context: &CpuContext, next_context: &CpuContext) {
        println!(
            "Switching from sp: {:#x} and ra: {:#x} to sp: {:#x} and ra:{:#x}",
            prev_context.sp,
            prev_context.ra,
            next_context.sp,
            next_context.ra,
        );

        unsafe {
            __switch_context(prev_context, next_context);
        }
    }
}
extern "C" {
    fn __switch_context(current: &CpuContext, to: &CpuContext);
}
global_asm!(
    "__switch_context:",
    "sw ra, 0(a0)",
    "sw sp, 4(a0)",
    "sw s0, 8(a0)",
    "sw s1, 12(a0)",
    "sw s2, 16(a0)",
    "sw s3, 20(a0)",
    "sw s4, 24(a0)",
    "sw s5, 28(a0)",
    "sw s6, 32(a0)",
    "sw s7, 36(a0)",
    "sw s8, 40(a0)",
    "sw s9, 44(a0)",
    "sw s10, 48(a0)",
    "sw s11, 52(a0)",
    "lw ra, 0(a1)",
    "lw sp, 4(a1)",
    "lw s0, 8(a1)",
    "lw s1, 12(a1)",
    "lw s2, 16(a1)",
    "lw s3, 20(a1)",
    "lw s4, 24(a1)",
    "lw s5, 28(a1)",
    "lw s6, 32(a1)",
    "lw s7, 36(a1)",
    "lw s8, 40(a1)",
    "lw s9, 44(a1)",
    "lw s10, 48(a1)",
    "lw s11, 52(a1)",
    "ret",
);
