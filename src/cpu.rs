//! What the host processor can do, asked at run time.
//!
//! `std::arch::is_aarch64_feature_detected!` and `std::is_x86_feature_detected!` are the two
//! reasons `cranelift-native` was not `#![no_std]`. There is no `core` equivalent: the macros are
//! `std_detect`, which `core` deliberately does not carry because the answer comes from the
//! operating system on every architecture except x86.
//!
//! So it comes from the operating system here instead. This is not a reimplementation of feature
//! detection in general; it is the six aarch64 predicates and the twenty x86-64 predicates that
//! the backend actually reads, and nothing else. Adding one is adding a field.
//!
//! **Not cached.** `std_detect` memoises because `is_x86_feature_detected!` can sit in a hot
//! loop; the only caller here is `cranelift_native::infer_native_flags`, which runs twice per
//! compilation unit. Six `sysctl` calls at that rate are not worth a static to get wrong. A
//! caller that does put this in a loop should cache it itself, or this should grow one.
//!
//! The bit positions and sysctl names are the ones `std_detect` uses, which are in turn the ones
//! the kernel headers and the Intel and AMD manuals document. A wrong bit here does not fail
//! loudly: it silently tells the backend a feature is present that is not, and the first symptom
//! is `SIGILL` in generated code. That is why each block names its source.

/// What the host aarch64 processor supports, of the features the backend asks about.
///
/// Fields, not methods, because every one of them costs a syscall on Darwin and the callers want
/// all of them at once.
#[cfg(target_arch = "aarch64")]
#[derive(Debug, Clone, Copy, Default)]
pub struct Aarch64Features {
    /// FEAT_LSE: the large-system atomics, `CAS`/`SWP`/`LDADD` and friends.
    pub lse: bool,
    /// FEAT_PAuth: pointer authentication, which is what `sign_return_address` needs.
    pub pauth: bool,
    /// FEAT_FP16: half-precision floating point as an arithmetic type.
    pub fp16: bool,
    /// FEAT_DotProd: `SDOT`/`UDOT`.
    pub dotprod: bool,
    /// FEAT_I8MM: the 8-bit integer matrix-multiply instructions.
    pub i8mm: bool,
    /// FEAT_BTI: branch target identification, which changes how pages are mapped executable.
    pub bti: bool,
}

#[cfg(target_arch = "aarch64")]
impl Aarch64Features {
    /// Ask the operating system.
    pub fn detect() -> Aarch64Features {
        detect_aarch64()
    }
}

/// Darwin publishes CPU features as `sysctl` nodes.
///
/// The names are the modern `hw.optional.arm.FEAT_*` spelling, which macOS has exposed since
/// 12.3. A node that does not exist returns an error, which reads as "not present" - the correct
/// answer on an older kernel for every feature the backend asks about, since none of them existed
/// on a Mac before Apple Silicon.
///
/// Source: Apple's "Determining Instruction Set Characteristics", and `std_detect`'s
/// `os/darwin/aarch64.rs`, which is where these exact names come from.
#[cfg(all(target_arch = "aarch64", target_vendor = "apple"))]
fn detect_aarch64() -> Aarch64Features {
    Aarch64Features {
        lse: sysctl_flag(c"hw.optional.arm.FEAT_LSE"),
        pauth: sysctl_flag(c"hw.optional.arm.FEAT_PAuth"),
        fp16: sysctl_flag(c"hw.optional.arm.FEAT_FP16"),
        dotprod: sysctl_flag(c"hw.optional.arm.FEAT_DotProd"),
        i8mm: sysctl_flag(c"hw.optional.arm.FEAT_I8MM"),
        bti: sysctl_flag(c"hw.optional.arm.FEAT_BTI"),
    }
}

/// Read one `sysctl` node as a boolean.
///
/// # Safety
///
/// `sysctlbyname` writes at most `len` bytes into the buffer and updates `len` with what it
/// wrote. The buffer is a local `i32` and `len` starts at its size, so the call cannot write past
/// it whatever the node's real width is.
#[cfg(all(target_arch = "aarch64", target_vendor = "apple"))]
fn sysctl_flag(name: &core::ffi::CStr) -> bool {
    let mut enabled: i32 = 0;
    let mut len: usize = core::mem::size_of::<i32>();
    let ok = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            (&raw mut enabled).cast(),
            &mut len,
            core::ptr::null_mut(),
            0,
        )
    };
    ok == 0 && enabled != 0
}

/// Linux publishes CPU features in the auxiliary vector.
///
/// Bit positions are `arch/arm64/include/uapi/asm/hwcap.h`. `fp16` is two bits, not one: the
/// scalar half-precision support (`FPHP`) and the base floating point unit both have to be there
/// before the type is usable.
#[cfg(all(target_arch = "aarch64", target_os = "linux"))]
fn detect_aarch64() -> Aarch64Features {
    const AT_HWCAP: libc::c_ulong = 16;
    const AT_HWCAP2: libc::c_ulong = 26;

    let hwcap = unsafe { libc::getauxval(AT_HWCAP) };
    let hwcap2 = unsafe { libc::getauxval(AT_HWCAP2) };
    let bit = |word: libc::c_ulong, n: u32| word & (1 << n) != 0;

    Aarch64Features {
        lse: bit(hwcap, 8),
        pauth: bit(hwcap, 30),
        fp16: bit(hwcap, 0) && bit(hwcap, 9),
        dotprod: bit(hwcap, 20),
        i8mm: bit(hwcap2, 13),
        bti: bit(hwcap2, 17),
    }
}

/// An aarch64 host that is neither Darwin nor Linux.
///
/// Every field false. That is the conservative answer and not a silent stub: claiming a feature
/// the processor lacks produces `SIGILL` in emitted code, whereas claiming none produces
/// baseline Armv8.0 code that runs everywhere. A port that wants the features adds its own
/// branch above.
#[cfg(all(
    target_arch = "aarch64",
    not(target_vendor = "apple"),
    not(target_os = "linux")
))]
fn detect_aarch64() -> Aarch64Features {
    Aarch64Features::default()
}

/// What the host x86-64 processor supports, of the features the backend asks about.
#[cfg(target_arch = "x86_64")]
#[derive(Debug, Clone, Copy, Default)]
pub struct X86Features {
    /// SSE2. Baseline for the target, but the backend still refuses to run without it.
    pub sse2: bool,
    /// `CMPXCHG16B`, which is how a 128-bit atomic is done without FEAT_LSE's equivalent.
    pub cmpxchg16b: bool,
    /// SSE3.
    pub sse3: bool,
    /// SSSE3.
    pub ssse3: bool,
    /// SSE4.1.
    pub sse41: bool,
    /// SSE4.2.
    pub sse42: bool,
    /// `POPCNT`.
    pub popcnt: bool,
    /// AVX. Requires the OS to have opted into saving the wide registers.
    pub avx: bool,
    /// AVX2.
    pub avx2: bool,
    /// FMA.
    pub fma: bool,
    /// AVX-VNNI, the VEX-encoded dot products.
    pub avx_vnni: bool,
    /// AVX-512 VNNI.
    pub avx512vnni: bool,
    /// BMI1.
    pub bmi1: bool,
    /// BMI2.
    pub bmi2: bool,
    /// AVX-512 BITALG.
    pub avx512bitalg: bool,
    /// AVX-512 DQ.
    pub avx512dq: bool,
    /// AVX-512 F, the foundation the rest of AVX-512 sits on.
    pub avx512f: bool,
    /// AVX-512 VL.
    pub avx512vl: bool,
    /// AVX-512 VBMI.
    pub avx512vbmi: bool,
    /// `LZCNT`, which is ABM on AMD.
    pub lzcnt: bool,
}

#[cfg(target_arch = "x86_64")]
impl X86Features {
    /// Ask the processor.
    pub fn detect() -> X86Features {
        detect_x86()
    }
}

/// x86 feature detection is the `CPUID` instruction and needs no operating system, which is why
/// this is the one architecture `core` could have carried and did not.
///
/// The leaves and bit positions are Intel SDM Volume 2 "CPUID", cross-checked against
/// `std_detect`'s `os/x86.rs`.
///
/// The `XCR0` dance is the part that is easy to get wrong and matters most. A processor can
/// report AVX while the operating system has not enabled saving the wide registers across a
/// context switch, and using AVX then corrupts state rather than trapping. So AVX and everything
/// above it is gated on `OSXSAVE` plus the `XCR0` bits, exactly as the SDM requires.
#[cfg(target_arch = "x86_64")]
fn detect_x86() -> X86Features {
    use core::arch::x86_64::{CpuidResult, __cpuid, __cpuid_count, _xgetbv};

    let mut f = X86Features::default();

    // `CPUID` is unconditionally present on x86-64, so every call below is defined.
    let CpuidResult { eax: max_leaf, .. } = unsafe { __cpuid(0) };
    if max_leaf < 1 {
        return f;
    }

    let CpuidResult { ecx: info_ecx, edx: info_edx, .. } = unsafe { __cpuid(1) };
    let (ext_ebx, ext_ecx, ext_eax_1) = if max_leaf >= 7 {
        let CpuidResult { ebx, ecx, .. } = unsafe { __cpuid(7) };
        let CpuidResult { eax, .. } = unsafe { __cpuid_count(7, 1) };
        (ebx, ecx, eax)
    } else {
        (0, 0, 0)
    };
    let CpuidResult { eax: max_ext_leaf, .. } = unsafe { __cpuid(0x8000_0000) };
    let ext_info_ecx = if max_ext_leaf >= 0x8000_0001 {
        let CpuidResult { ecx, .. } = unsafe { __cpuid(0x8000_0001) };
        ecx
    } else {
        0
    };

    let bit = |word: u32, n: u32| word & (1 << n) != 0;

    f.sse3 = bit(info_ecx, 0);
    f.ssse3 = bit(info_ecx, 9);
    f.cmpxchg16b = bit(info_ecx, 13);
    f.sse41 = bit(info_ecx, 19);
    f.sse42 = bit(info_ecx, 20);
    f.popcnt = bit(info_ecx, 23);
    f.sse2 = bit(info_edx, 26);
    f.bmi1 = bit(ext_ebx, 3);
    f.bmi2 = bit(ext_ebx, 8);
    f.lzcnt = bit(ext_info_ecx, 5);

    let f16c = bit(info_ecx, 29);
    let xsave = bit(info_ecx, 26);
    let osxsave = bit(info_ecx, 27);
    if !(xsave && osxsave) {
        return f;
    }

    // Reading `XCR0` is defined here and only here: `XGETBV` raises #UD unless `CR4.OSXSAVE` is
    // set, which is exactly what the `osxsave` bit reports.
    let xcr0 = unsafe { _xgetbv(0) };
    // `XCR0.SSE[1]` and `XCR0.AVX[2]`.
    let os_avx = xcr0 & 0b110 == 0b110;
    // `XCR0.opmask[5]`, `XCR0.ZMM_hi256[6]`, `XCR0.Hi16_ZMM[7]`.
    let os_avx512 = xcr0 & 0xe0 == 0xe0;
    if !os_avx {
        return f;
    }

    f.fma = bit(info_ecx, 12);
    f.avx = bit(info_ecx, 28);
    f.avx2 = bit(ext_ebx, 5);
    f.avx_vnni = bit(ext_eax_1, 4);

    // Rust's own detection makes AVX-512F imply FMA and F16C, on the grounds that the instruction
    // set does not formally require them but no processor ships one without the others. The
    // backend inherits that, so that a flag set here means the same thing it meant before.
    if os_avx512 && f16c && f.fma {
        f.avx512f = bit(ext_ebx, 16);
        f.avx512dq = bit(ext_ebx, 17);
        f.avx512vl = bit(ext_ebx, 31);
        f.avx512vbmi = bit(ext_ecx, 1);
        f.avx512vnni = bit(ext_ecx, 11);
        f.avx512bitalg = bit(ext_ecx, 12);
    }

    f
}
