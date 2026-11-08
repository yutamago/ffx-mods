// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;
using System.Threading;

namespace Yuta.FfxMods.Lighting;

/// <summary>
/// render.lighting: the PS2's character lighting, hooks on FFX.exe's PhyreEngine side. Addresses are
/// ffx-rust's symbols/ffx.sym virtual addresses (image base 0x00400000); notes in docs/render.md.
/// </summary>
internal static unsafe class LightingPatches {
    private const uint IMAGE_BASE = 0x00400000;

    private static FhLogger? _log;
    private static int _logged;

    /// <summary>Logs a hook's first call (bit = which hook), to show in the log that it runs.</summary>
    private static void first(int bit, string message) {
        if ((Interlocked.Or(ref _logged, bit) & bit) == 0) _log?.Info(message);
    }

    private static FhCallHandle<T> at<T>(uint va) where T : Delegate =>
        new(new FhCallLocation("FFX.exe", (nint)(va - IMAGE_BASE)));

    // ---- render.lighting --------------------------------------------------------------------

    /// <summary>
    /// Phyre_CharacterLighting_applyIntensityMultiplier: thiscall(cl; i, flag) ret 8. Sets light i's
    /// level (PLight +0x24; the shader's Light_i.m_colorIntensity is colour x level) to
    /// (1 - glow factor) x 1.2 x map intensity x 1.16 x 0.95, and for i = 3 the ambient level
    /// (+0x1a4, x 1.05 instead) and the ambient colour x level into the lighting record +0xac
    /// (CharacterAmbientLight). On znkd08/09/10/14 it also saturates the colours (x 2.6).
    /// Every light colour change (the PS2 code's Sg_LightSetColor) goes through it.
    /// </summary>
    [UnmanagedFunctionPointer(CallingConvention.ThisCall)]
    public delegate nint ApplyIntensityMultiplier(nint cl, int i, int flag);

    /// <summary>CharacterLighting setRotation: thiscall(cl; i, pitch, yaw) ret 0xc, the HD maps' per-map turn of light i (degrees).</summary>
    [UnmanagedFunctionPointer(CallingConvention.ThisCall)]
    public delegate void SetLightRotation(nint cl, int i, float pitch, float yaw);

    private const uint APPLY_INTENSITY_MULTIPLIER = 0x006a6e90;
    private const uint SET_LIGHT_ROTATION = 0x006aaea0;
    private static ApplyIntensityMultiplier? _intensity;
    private static SetLightRotation? _rotation;

    /// <summary>
    /// The level of every character light and of the ambient light. The PS2's VU1 program turns
    /// the lit colour (colour matrix x max(0, light matrix x N) + ambient, the script colours as
    /// byte / 255) into the vertex colour by vf8 = material colour 0x7f x shade x 2
    /// (FUN_008437f0 in FFX.exe, the leftover packet code), and the GS modulates the texture by
    /// vertex colour / 128: the texture x 254/128 x lit. docs/render.md.
    /// </summary>
    private const float PS2_LEVEL = 254f / 128f;

    private static float luminance(float* rgb) => (rgb[0] * 0.3f + rgb[1] * 0.5f + rgb[2] * 0.2f) * PS2_LEVEL;

    /// <summary>
    /// The f64 0.208125 that Phyre_CharacterLighting_setShadeCoeff (0x006aaef0, its only reader)
    /// multiplies the ambient intensity by into CharacterShadeCoeff: the shaders add
    /// CharacterShadeColor x CharacterShadeCoeff to every lit pixel, an HD fill light.
    /// </summary>
    private const uint SHADE_COEFF_SCALE = 0x00b48300;

    /// <summary>
    /// Ch_DrawMain's light-value term (0x0082f64c, with g_ChrShadeByLight, on in the shipped exe):
    /// fld [chr+0x36c]; fmul qword [0.85]; fadd qword [0.15]; the HD keeps 15% of the shade when a
    /// script dims a character's light value. The PS2 (Ch_DrawCalcShade) multiplies by it as it is.
    /// </summary>
    private const uint LIGHT_VALUE_SCALE = 0x0082f652;
    private const uint K_085 = 0x00b501a8;
    private const uint K_015 = 0x00b59870;

    /// <summary>
    /// Characters lit the PS2's way: the three directional lights and the ambient light at the
    /// PS2's level (no HD intensity, glow balance or Zanarkand saturation), no HD per-map rotation
    /// of the light directions, no HD shade-coefficient fill light, the script's light value as a
    /// plain multiplier. The PS2's falloff (max(0, N.L)) is in the shader files of the
    /// render.lighting converter job (docs/render.md).
    /// </summary>
    public static bool lighting(ModHost m, string value) {
        _log = m.log;
        if (!Native.patch_code(SHADE_COEFF_SCALE, [0, 0, 0, 0x10, 0xd7, 0xa3, 0xca, 0x3f], new byte[8], m.log))
            m.log.Warning("render.lighting: the HD shade coefficient stays");
        byte[] k085 = BitConverter.GetBytes(Native.reloc(K_085));
        byte[] k015 = BitConverter.GetBytes(Native.reloc(K_015));
        byte[] original = [0xdc, 0x0d, .. k085, 0xdc, 0x05, .. k015];
        if (!Native.patch_code(LIGHT_VALUE_SCALE, original, [0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90], m.log))
            m.log.Warning("render.lighting: the HD light value scale (0.85 v + 0.15) stays");
        _intensity = (cl, i, flag) => {
            first(2, $"render.lighting: light levels set to {PS2_LEVEL} (first call: light {i})");
            nint r = at<ApplyIntensityMultiplier>(APPLY_INTENSITY_MULTIPLIER).chain_from(_intensity!).fnptr!(cl, i, flag);
            byte* c = (byte*)cl;
            if (i is >= 0 and < 3) {
                nint set = *(nint*)(c + 0xf0);
                if (set == 0) return r;
                nint lights = *(nint*)((byte*)set + 4);
                if (lights == 0) return r;
                nint light = *(nint*)((byte*)lights + i * 4);
                if (light == 0) return r;
                *(float*)((byte*)light + 0x24) = PS2_LEVEL;
                *(float*)(c + 0x1d0 + i * 4) = luminance((float*)(c + 0x124 + i * 0x10));
            }
            else if (i == 3) {
                *(float*)(c + 0x1a4) = PS2_LEVEL;
                float* ambient = (float*)(c + 0x190);
                nint record = *(nint*)(c + 0x18c);
                if (record != 0) {
                    float* dst = (float*)((byte*)record + 0xac);
                    dst[0] = ambient[0] * PS2_LEVEL;
                    dst[1] = ambient[1] * PS2_LEVEL;
                    dst[2] = ambient[2] * PS2_LEVEL;
                }
                *(float*)(c + 0x1dc) = luminance(ambient);
            }
            return r;
        };
        if (!point_lights(m))
            m.log.Warning("render.lighting: point lights stay on every character");
        _rotation = (cl, i, pitch, yaw) => at<SetLightRotation>(SET_LIGHT_ROTATION).chain_from(_rotation!).fnptr!(cl, i, 0f, 0f);
        return at<ApplyIntensityMultiplier>(APPLY_INTENSITY_MULTIPLIER).hook(m, _intensity)
            && at<SetLightRotation>(SET_LIGHT_ROTATION).hook(m, _rotation);
    }

    /// <summary>
    /// Ch_PLightCalc (0x008305a0, from Ch_DrawMain): after summing the point lights in range into
    /// the character's four light slots (colours chr +0x384 + 16 i, directions +0x3c4 + 16 i) it
    /// hands the slot of the last light in range (edi = its colour, ebx = chr) to the shader's
    /// extra light (graphicCharacterLightingSetLightMatrix; LgtMatClr, applied as
    /// max(colour x half-Lambert^2, 1)) when bVar2 ([ebp-0x9c]) is set, else white light from
    /// above (0x00830a80), which changes nothing. The PS2 draw (Sg_LightSetChr, PS2 0x00106810)
    /// uses slot i only where bit i of the character's point-light byte (PC +0x188, PS2 +0x194)
    /// is set, else the global lights; the PC never reads that byte for drawing.
    /// </summary>
    private const uint PLIGHT_TEST = 0x008309b4;   // cmp dword [ebp-0x9c], 0; je 0x00830a80
    private const uint PLIGHT_USE = 0x008309c1;    // the slot goes to the shader
    private const uint PLIGHT_NONE = 0x00830a80;   // white light from above (no point light)

    /// <summary>
    /// The PC's point light reaches a character only where the PS2 would use that slot: a stub
    /// keeps Ch_PLightCalc's own test and takes the no-light branch unless bit slot of chr +0x188
    /// is set. Native code only: Ch_DrawMain may run on the job threads, so no managed code runs
    /// in it (an earlier managed hook on Ch_PLightCalc was suspected of crashes).
    /// </summary>
    private static bool point_lights(ModHost m) {
        uint use = Native.reloc(PLIGHT_USE), none = Native.reloc(PLIGHT_NONE), site = Native.reloc(PLIGHT_TEST);
        uint s = Native.alloc_code(46, a => [
            0x83, 0xbd, 0x64, 0xff, 0xff, 0xff, 0x00,                  // cmp dword [ebp-0x9c], 0
            0x0f, 0x84, .. Native.rel32(a + 13, none),                 // je none
            0x8b, 0xc7,                                                // mov eax, edi
            0x2b, 0xc3,                                                // sub eax, ebx
            0x2d, 0x84, 0x03, 0x00, 0x00,                              // sub eax, 0x384
            0xc1, 0xe8, 0x04,                                          // shr eax, 4: the slot
            0x0f, 0xb6, 0x8b, 0x88, 0x01, 0x00, 0x00,                  // movzx ecx, byte [ebx+0x188]
            0x0f, 0xa3, 0xc1,                                          // bt ecx, eax
            0x0f, 0x83, .. Native.rel32(a + 41, none),                 // jnc none
            0xe9, .. Native.rel32(a + 46, use),                        // jmp use
        ]);
        if (s == 0) return false;
        byte[] original = [0x83, 0xbd, 0x64, 0xff, 0xff, 0xff, 0x00, 0x0f, 0x84, 0xbf, 0x00, 0x00, 0x00];
        byte[] patched = [0xe9, .. Native.rel32(site + 5, s), 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90];
        if (!Native.patch_code(PLIGHT_TEST, original, patched, m.log)) return false;
        m.log.Info($"render.lighting: point lights only where the character's slot bit is set (stub 0x{s:x8})");
        return true;
    }
}
