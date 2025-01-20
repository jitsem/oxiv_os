use crate::println;
use core::arch::global_asm;
use core::fmt::Display;

const MAX_PROCESSES: usize = 2;

#[repr(C)]
pub struct CpuContext {
    // ra + sp + s0 ~ s11
    pub registers: [usize; 14],
}

pub enum ContextRegisters {
    Ra = 0,
    Sp = 1,
}

#[repr(C, align(16))]
pub struct Process {
    pid: u32,
    state: ProcessState,
    context: CpuContext,
    kernel_stack: [u8; 8192], // Kernel stack
}
impl Default for Process {
    fn default() -> Self {
        Self {
            pid: 0,
            state: ProcessState::Unused,
            kernel_stack: [0; 8192], // Explicitly initialize the array
            context: CpuContext { registers: [0; 14] },
        }
    }
}

pub struct ProcessInfo<'a> {
    process: &'a Process,
}

impl<'a> ProcessInfo<'a> {
    pub fn from(process: &'a Process) -> Self {
        ProcessInfo { process }
    }
}
impl Display for ProcessInfo<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "proc with id {}({:?}): {:#x} ",
            self.process.pid,
            self.process.state,
            self.process.context.registers[ContextRegisters::Sp as usize]
        )
    }
}

#[derive(PartialEq, Eq, Default, Debug, Clone, Copy)]
enum ProcessState {
    #[default]
    Unused,
    Runnable,
    Dead,
}

pub struct ProcessCreator<'a> {
    processes: [Process; MAX_PROCESSES],
    next_proc_id: u32,
    current_running: Option<&'a Process>,
    idle_process: Process,
}

impl<'a> ProcessCreator<'a> {
    pub const fn new() -> Self {
        //TODO the entire idle_process thing can be more elegant
        let idle_process = Process {
            pid: 0,
            state: ProcessState::Dead,
            kernel_stack: [0; 8192],
            context: CpuContext { registers: [0; 14] },
        };
        ProcessCreator {
            processes: [
                Process {
                    pid: 0,
                    state: ProcessState::Unused,
                    kernel_stack: [0; 8192],
                    context: CpuContext { registers: [0; 14] },
                },
                Process {
                    pid: 0,
                    state: ProcessState::Unused,
                    kernel_stack: [0; 8192],
                    context: CpuContext { registers: [0; 14] },
                },
            ],
            next_proc_id: 1,
            current_running: None,
            idle_process,
        }
    }

    /// TODO: Make this mandatory
    pub fn init(&'a mut self) {
        Self::init_process(&mut self.idle_process, 0xFFFFFFFF);
        self.current_running = Some(&self.idle_process);
    }

    pub fn create_process(&mut self, entry_point: usize) -> ProcessInfo {
        let mut available_proc: Option<&mut Process> = None;
        for proc in self.processes.iter_mut() {
            if proc.state == ProcessState::Unused {
                available_proc = Some(proc);
            }
        }
        if let Some(available_proc) = available_proc {
            available_proc.pid = self.next_proc_id;
            self.next_proc_id += 1;
            available_proc.state = ProcessState::Runnable;
            Self::init_process(available_proc, entry_point);
            // Update the process struct with the new stack pointer
            ProcessInfo::from(available_proc)
        } else {
            panic!("This kernel only supports 8 processes!")
        }
    }

    fn init_process(proc: &mut Process, entry_point: usize) {
        unsafe {
            let sp = proc.kernel_stack.as_mut_ptr().add(proc.kernel_stack.len());
            assert!(
                sp as usize % 16 == 0,
                "stack_pointer is not 16-byte aligned"
            );
            proc.context.registers[ContextRegisters::Sp as usize] = sp as usize;
            proc.context.registers[ContextRegisters::Ra as usize] = entry_point;
        }
    }

    pub extern "C" fn yield_control(&'a mut self) {
        let (prev_pid, prev_context) = match self.current_running {
            None => (self.idle_process.pid, &self.idle_process.context),
            Some(proc) => (proc.pid, &proc.context),
        };

        let mut found_processes = self
            .processes
            .iter()
            .filter(|p| p.pid != prev_pid && p.state == ProcessState::Runnable);

        if let Some(proc) = found_processes.nth(0) {
            println!(
                "Switching from {} to {}",
                self.current_running.unwrap().pid,
                proc.pid
            );
            self.current_running = Some(proc);
            let next_context = &self.current_running.unwrap().context;
            Self::switch_context(prev_context, next_context);
        } else {
            println!("Couldn't yield to other processes");
        }
    }

    #[no_mangle]
    extern "C" fn switch_context(prev_context: &CpuContext, next_context: &CpuContext) {
        println!(
            "Switching from sp: {:#x} and ra: {:#x} to sp: {:#x} and ra:{:#x}",
            prev_context.registers[ContextRegisters::Sp as usize],
            prev_context.registers[ContextRegisters::Ra as usize],
            next_context.registers[ContextRegisters::Sp as usize],
            next_context.registers[ContextRegisters::Ra as usize],
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
