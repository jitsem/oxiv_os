use super::process::{CpuContext, Process, ProcessState};
use crate::page_table::PageTable;
use crate::{arch, println};
use alloc::{boxed::Box, collections::vec_deque::VecDeque};
use core::arch::asm;
use core::fmt::Display;
use core::ptr::null;

const MAX_PROCESSES: usize = 2;

pub struct Scheduler {
    pub processes: VecDeque<Process>,
    next_proc_id: u32,
    pub current_running: Option<Process>,
    pub previously_running: Option<Process>,
    root_page_table: *const PageTable,
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
            "proc with id {}({:?}): sp={:#x}",
            self.pid, self.state, self.stack_pointer
        )
    }
}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            processes: VecDeque::new(),
            next_proc_id: 1,
            current_running: None,
            previously_running: None,
            root_page_table: null(),
        }
    }

    pub fn init(&mut self, page_table_addr: *const PageTable) {
        self.root_page_table = page_table_addr;
        self.processes.reserve(MAX_PROCESSES);
        self.current_running = Some(self.create_idle_process());
    }

    /// Schedule a U-mode process with its own page table and user stack.
    pub fn schedule_user_process(
        &mut self,
        entry_point: usize,
        page_table_addr: usize,
        user_stack_top: usize,
    ) -> ProcessInfo {
        let mut new_proc = Process {
            pid: self.next_proc_id,
            state: ProcessState::Runnable,
            page_table_addr,
            kernel_stack: Box::new([0; 8192]),
            kernel_sp_top: 0,
            context: CpuContext::default(),
        };
        println!(
            "Process {}: kernel_stack at {:p}",
            new_proc.pid,
            new_proc.kernel_stack.as_ptr()
        );
        self.next_proc_id += 1;
        Self::init_user_process(&mut new_proc, entry_point, user_stack_top);
        let info = ProcessInfo::from(&new_proc);
        self.processes.push_back(new_proc);
        info
    }

    /// Set up a fake TrapFrame on the kernel stack so the first sret starts the process
    /// at entry_point in U-mode with user_stack_top as the stack pointer.
    fn init_user_process(proc: &mut Process, entry_point: usize, user_stack_top: usize) {
        unsafe {
            let kernel_top = proc.kernel_stack.as_mut_ptr().add(proc.kernel_stack.len()) as usize;
            proc.kernel_sp_top = kernel_top;

            // Place a zeroed 132-byte TrapFrame (33 words) just below the kernel stack top.
            let frame_base = (kernel_top - 132) as *mut u32;
            for i in 0..33usize {
                frame_base.add(i).write(0);
            }
            // word 30 (offset 120) = sp: user stack top
            frame_base.add(30).write(user_stack_top as u32);
            // word 31 (offset 124) = sepc: entry point
            frame_base.add(31).write(entry_point as u32);
            // word 32 (offset 128) = sstatus: SPIE=1 (bit 5), SPP=0 → sret enters U-mode
            frame_base.add(32).write(1u32 << 5);

            proc.context.sp = kernel_top - 132;
        }
    }

    /// Called from handle_trap to perform a context switch.
    /// Saves current_frame_ptr, picks the next runnable process, switches the page table,
    /// updates sscratch, and returns the next TrapFrame pointer.
    pub fn prepare_next_process(&mut self, current_frame_ptr: usize) -> usize {
        // Save the current kernel-stack position into the current process
        if let Some(current) = self.current_running.as_mut() {
            current.context.sp = current_frame_ptr;
        }

        // Re-queue previously_running if still runnable
        if let Some(prev) = self.previously_running.take() {
            if prev.state == ProcessState::Runnable {
                self.processes.push_back(prev);
            }
        }

        // Current becomes previously_running
        self.previously_running = self.current_running.take();

        // Pick next
        let next = self.processes.pop_front();
        self.current_running = match next {
            None if self
                .previously_running
                .as_ref()
                .is_some_and(|p| p.state == ProcessState::Runnable) =>
            {
                self.previously_running.take()
            }
            None => {
                panic!("No runnable processes (kernel idle)");
            }
            Some(p) => Some(p),
        };

        let next_proc = self.current_running.as_ref().unwrap();
        println!(
            "Switching to process {} (frame={:#x})",
            next_proc.pid, next_proc.context.sp
        );

        // Install next process's page table
        arch::Satp::new(next_proc.page_table_addr).switch();

        // Set sscratch so the next trap from U-mode gets the right kernel stack
        unsafe {
            asm!("csrw sscratch, {}", in(reg) next_proc.kernel_sp_top);
        }

        next_proc.context.sp
    }

    fn create_idle_process(&self) -> Process {
        let kernel_stack = Box::new([0u8; 8192]);
        let kernel_top = unsafe { kernel_stack.as_ptr().add(kernel_stack.len()) as usize };
        Process {
            pid: 0,
            state: ProcessState::KernelReserved,
            page_table_addr: self.root_page_table as usize,
            kernel_sp_top: kernel_top,
            kernel_stack,
            context: CpuContext::default(),
        }
    }
}
