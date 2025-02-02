use core::arch::asm;

#[derive(Default)]
struct SbiArgs {
    arg0: u32,
    arg1: u32,
    arg2: u32,
    arg3: u32,
    arg4: u32,
    arg5: u32,
    fid: u32,
    eid: u32,
}

/// Not used at the moment
#[allow(dead_code)]
struct SbiResult {
    error: u32,
    value: u32,
}

pub struct Sbi;
impl Sbi {
    fn call(args: &SbiArgs) -> SbiResult {
        let mut a0 = args.arg0;
        let mut a1 = args.arg1;

        //Safety: Inline assembly is inherently unsafe.
        //We are doing an ecall to the SBI firmware here.
        unsafe {
            asm!(
                "ecall",
                inout("a0") a0,
                inout("a1") a1,
                in("a2") args.arg2,
                in("a3") args.arg3,
                in("a4") args.arg4,
                in("a5") args.arg5,
                in("a6") args.fid,
                in("a7") args.eid,
                options(nostack, preserves_flags),
            );
        }

        SbiResult {
            error: a0,
            value: a1,
        }
    }

    pub fn put_char(to_write: char) {
        let args = SbiArgs {
            arg0: to_write as u32,
            fid: 0,
            eid: 1,
            ..Default::default()
        };
        let _ = Sbi::call(&args);
    }
}
