use alloc::boxed::Box;

#[repr(C, align(16))]
#[derive(Clone, Default)]
pub struct CpuContext {
    pub ra: usize,
    pub sp: usize,
    pub s0: usize,
    pub s1: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
}

#[repr(C)]
#[derive(Clone)]
pub struct Process {
    pub pid: u32,
    pub state: ProcessState,
    pub context: CpuContext,
    pub kernel_stack: Box<[u8; 8192]>, //We allocate, but don't use directly. Used via pointer/assembly magic.
}
impl Default for Process {
    fn default() -> Self {
        Self {
            pid: 0,
            state: ProcessState::Unused,
            kernel_stack: Box::new([0; 8192]),
            context: CpuContext::default(),
        }
    }
}

#[derive(PartialEq, Eq, Default, Debug, Clone, Copy)]
pub enum ProcessState {
    #[default]
    Unused,
    Runnable,
    Exited,
    KernelReserved,
}
