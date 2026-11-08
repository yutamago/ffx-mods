// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;

namespace Yuta.Demaster;

/// <summary>
/// FFX.exe's memory, addressed by the VAs of ffx-rust's symbols/ffx.sym (image base 0x00400000).
/// Fahrenheit takes offsets from the module base instead: <see cref="off"/>.
/// </summary>
internal static unsafe class Native {
    private const nint IMAGE_BASE = 0x00400000;
    private const uint PAGE_EXECUTE_READWRITE = 0x40;

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool VirtualProtect(nint address, nuint size, uint protect, out uint old);

    [DllImport("kernel32.dll")]
    private static extern bool FlushInstructionCache(nint process, nint address, nuint size);

    [DllImport("kernel32.dll")]
    private static extern nint GetCurrentProcess();

    /// <summary>The module-base offset of an ffx.sym address, as FhUtil and FhCallLocation take it.</summary>
    public static nint off(uint va) => (nint)va - IMAGE_BASE;

    public static byte* ptr(uint va) => FhUtil.ptr_at<byte>(off(va));

    /// <summary>An ffx.sym VA as an absolute address in this process (FFX.exe is relocated).</summary>
    public static uint reloc(uint va) => (uint)(nint)ptr(va);

    public static byte  u8 (uint va) => *ptr(va);
    public static sbyte i8 (uint va) => *(sbyte*)ptr(va);
    public static short i16(uint va) => *(short*)ptr(va);
    public static uint  u32(uint va) => *(uint*)ptr(va);
    public static void  set_u8(uint va, byte v) => *ptr(va) = v;

    /// <summary>
    /// Writes <paramref name="patched"/> over code at <paramref name="va"/> when the bytes there are
    /// <paramref name="original"/> (the FFX.exe this mod was written for). Returns false and writes
    /// nothing when they differ, so another game build or another mod's patch is left alone.
    /// </summary>
    public static bool patch_code(uint va, byte[] original, byte[] patched, FhLogger log) {
        if (original.Length != patched.Length) throw new ArgumentException("patch length");
        byte* p = ptr(va);
        ReadOnlySpan<byte> now = new(p, original.Length);
        if (now.SequenceEqual(patched)) return true;
        if (!now.SequenceEqual(original)) {
            log.Warning($"0x{va:x8}: expected {Convert.ToHexString(original)}, found {Convert.ToHexString(now)}; not patched");
            return false;
        }
        if (!VirtualProtect((nint)p, (nuint)patched.Length, PAGE_EXECUTE_READWRITE, out uint old)) {
            log.Warning($"0x{va:x8}: VirtualProtect failed ({Marshal.GetLastWin32Error()})");
            return false;
        }
        patched.CopyTo(new Span<byte>(p, patched.Length));
        VirtualProtect((nint)p, (nuint)patched.Length, old, out _);
        FlushInstructionCache(GetCurrentProcess(), (nint)p, (nuint)patched.Length);
        return true;
    }

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern nint VirtualAlloc(nint address, nuint size, uint type, uint protect);

    /// <summary>
    /// Allocates <paramref name="size"/> bytes of executable memory, fills them with
    /// <paramref name="build"/>(their address) and returns the address (0 on failure). For stubs
    /// that a code patch jumps to: they run without managed code, so they are safe on the game's
    /// job threads. Never freed (they live as long as the game).
    /// </summary>
    public static uint alloc_code(int size, Func<uint, byte[]> build) {
        nint p = VirtualAlloc(0, (nuint)size, 0x3000 /* MEM_COMMIT | MEM_RESERVE */, PAGE_EXECUTE_READWRITE);
        if (p == 0) return 0;
        byte[] code = build((uint)p);
        if (code.Length != size) throw new ArgumentException($"stub is {code.Length} bytes, not {size}");
        code.CopyTo(new Span<byte>((void*)p, size));
        FlushInstructionCache(GetCurrentProcess(), p, (nuint)size);
        return (uint)p;
    }

    /// <summary>The rel32 of a jmp/jcc whose 4-byte operand ends at <paramref name="next"/> (absolute addresses).</summary>
    public static byte[] rel32(uint next, uint target) => BitConverter.GetBytes((int)(target - next));

    /// <summary>A hookable or callable function at an ffx.sym VA.</summary>
    public static FhCallHandle<T> fn<T>(uint va) where T : Delegate => new(new FhCallLocation("FFX.exe", off(va)));
}

/// <summary>Function shapes of the hooked and called game functions (all cdecl).</summary>
internal static class Fn {
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate void Void();
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate byte RetByte();
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] public delegate byte AnalogHist(int pad, int port, int axis, int hist);
}
