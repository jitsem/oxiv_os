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

        //UNSAFE: This is how we call the SBI system calls.
        unsafe {
            asm!(
                "ecall",
                inout("a0") a0,  // a0 serves as both input and output
                inout("a1") a1,  // a1 serves as both input and output
                in("a2") args.arg2,
                in("a3") args.arg3,
                in("a4") args.arg4,
                in("a5") args.arg5,
                in("a6") args.fid,
                in("a7") args.eid,    // Function ID and extension ID
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

    pub fn put_val(to_write: u32) {
        let args = SbiArgs {
            arg0: to_write,
            fid: 0,
            eid: 1,
            ..Default::default()
        };
        let _ = Sbi::call(&args);
    }
}
