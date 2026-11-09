// SPDX-License-Identifier: MIT

using System.Runtime.InteropServices;
using System.Threading;

namespace Yuta.FfxMods.Characters;

/// <summary>
/// render.texture_filter and render.texture_quality: how the PS2 textures are sampled and loaded.
/// Addresses are ffx-rust's symbols/ffx.sym virtual addresses (image base 0x00400000); notes in
/// docs/render.md.
/// </summary>
internal static unsafe class TexturePatches {
    private const uint IMAGE_BASE = 0x00400000;

    private static FhLogger? _log;
    private static int _logged;

    /// <summary>Logs a hook's first call (bit = which hook), to show in the log that it runs.</summary>
    private static void first(int bit, string message) {
        if ((Interlocked.Or(ref _logged, bit) & bit) == 0) _log?.Info(message);
    }

    private static FhCallHandle<T> at<T>(uint va) where T : Delegate =>
        new(new FhCallLocation("FFX.exe", (nint)(va - IMAGE_BASE)));

    // ---- render.texture_filter --------------------------------------------------------------

    /// <summary>
    /// Phyre_PSerialization_PBinary_loadCluster: cdecl(stream, cluster, path, key) -> 0 or an
    /// error. Every Direct3D object of a cluster (textures, buffers, shaders, its samplers) is made
    /// inside this call, on the calling thread (ffx-rust docs/formats/phyre.md).
    /// </summary>
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate int LoadCluster(nint stream, nint cluster, nint path, nint key);

    /// <summary>
    /// PSamplerStateD3D11_create: thiscall(sampler) -> 0/0xe/0x16. Fills a D3D11_SAMPLER_DESC from
    /// the PSamplerState (+0 min filter, +1 mag filter, +2..+4 address modes, +0x1c bit 6
    /// comparison) and calls ID3D11Device::CreateSamplerState.
    /// </summary>
    [UnmanagedFunctionPointer(CallingConvention.ThisCall)]
    public delegate int CreateSampler(nint sampler);

    /// <summary>
    /// PRendererD3D11_mapSamplerFilter: cdecl(min, mag, comparison) -> D3D11_FILTER. Returns
    /// ANISOTROPIC (0x55) for every sampler when the Anisotropic option (the f32 at 0x00c25370)
    /// is above 1, whatever the filters say.
    /// </summary>
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    public delegate int MapSamplerFilter(int min, int mag, int comparison);

    private const uint LOAD_CLUSTER = 0x0057bce0;
    private const uint CREATE_SAMPLER = 0x00595bf0;
    private const uint MAP_SAMPLER_FILTER = 0x0058aa10;
    /// <summary>D3D11_FILTER_MIN_MAG_POINT_MIP_LINEAR: texels unfiltered, mip levels blended.</summary>
    private const int FILTER_POINT_MIP_LINEAR = 0x01;

    private static LoadCluster? _load_cluster;
    private static CreateSampler? _create_sampler;
    private static MapSamplerFilter? _map_filter;
    [ThreadStatic] private static string? _cluster;
    [ThreadStatic] private static bool _point;
    private static readonly HashSet<string> _logged_clusters = [];

    /// <summary>
    /// The clusters whose samplers get point sampling: models, maps and effects, the 3D scene.
    /// The 2D rooms' paintings (map/.../2d/), menus, fonts and the PC's own screens keep theirs;
    /// samplers the engine makes outside cluster loads (post-process, shadows, glow, text) too.
    /// </summary>
    private static bool scene_cluster(string path) {
        string p = path.Replace('\\', '/').ToLowerInvariant();
        if (p.Contains("/2d/") || p.Contains("/menu") || p.Contains("/font") || p.Contains("/shaders/")) return false;
        return p.Contains("/chr/") || p.Contains("/map/") || p.Contains("/btlmap/") || p.Contains("/battle/")
            || p.Contains("/effect/") || p.Contains("/magic/") || p.Contains("/event/") || p.Contains("/obj/");
    }

    /// <summary>
    /// Nearest-neighbour texels on the 3D scene's materials: their samplers are made with
    /// MIN_MAG_POINT_MIP_LINEAR instead of the game's (anisotropic or trilinear) filter.
    /// </summary>
    public static bool texture_filter(ModHost m, string value) {
        _log = m.log;
        if (!cluster_hook(m)) return false;
        _create_sampler = sampler => {
            byte* s = (byte*)sampler;
            bool point = _cluster is { } path && (s[0x1c] & 0x40) == 0 && scene_cluster(path);
            // One line for the first sampler of each of the first 200 clusters (and of the engine's own).
            bool first_of_cluster;
            lock (_logged_clusters) first_of_cluster = _logged_clusters.Count < 200 && _logged_clusters.Add(_cluster ?? "");
            if (first_of_cluster)
                _log?.Info($"render.texture_filter: sampler min {s[0]} mag {s[1]} address {s[2]} {s[3]} {s[4]} flags {s[0x1c]:x2} cluster '{_cluster}' -> {(point ? "point" : "kept")}");
            _point = point;
            try {
                return at<CreateSampler>(CREATE_SAMPLER).chain_from(_create_sampler!).fnptr!(sampler);
            }
            finally {
                _point = false;
            }
        };
        _map_filter = (min, mag, comparison) => _point
            ? FILTER_POINT_MIP_LINEAR
            : at<MapSamplerFilter>(MAP_SAMPLER_FILTER).chain_from(_map_filter!).fnptr!(min, mag, comparison);
        return at<CreateSampler>(CREATE_SAMPLER).hook(m, _create_sampler)
            && at<MapSamplerFilter>(MAP_SAMPLER_FILTER).hook(m, _map_filter);
    }

    /// <summary>
    /// The loadCluster hook that keeps the path of the cluster being loaded in <see cref="_cluster"/>
    /// (per thread; loads nest), shared by render.texture_filter and render.texture_quality and
    /// installed once.
    /// </summary>
    private static bool cluster_hook(ModHost m) {
        if (_load_cluster is not null) return true;
        _load_cluster = (stream, cluster, path, key) => {
            string? outer = _cluster;
            bool? outer_ours = _ours;
            _cluster = path == 0 ? "" : Marshal.PtrToStringAnsi(path) ?? "";
            _ours = null;
            try {
                return at<LoadCluster>(LOAD_CLUSTER).chain_from(_load_cluster!).fnptr!(stream, cluster, path, key);
            }
            finally {
                _cluster = outer;
                _ours = outer_ours;
            }
        };
        return at<LoadCluster>(LOAD_CLUSTER).hook(m, _load_cluster);
    }

    // ---- render.texture_quality -------------------------------------------------------------

    /// <summary>
    /// Phyre_PRenderInterface_getTextureQuality: thiscall(render interface 0x00c94f00) -> +0x1340,
    /// the TextureQuality of GameSetting.ini (0 TQ_HIGH, 1 TQ_MEDIUM, 2 TQ_LOW). Its callers,
    /// Phyre_PBinary_loadClusterTextures (inside loadCluster) and
    /// Phyre_PBinary_reloadClusterTextures, drop that many top mip levels of every texture with a
    /// chain (ffx-rust docs/formats/phyre.md, "How the game loads a texture"). 7 bytes:
    /// mov eax, [ecx+0x1340]; ret.
    /// </summary>
    [UnmanagedFunctionPointer(CallingConvention.ThisCall)]
    public delegate int GetTextureQuality(nint render);

    private const uint GET_TEXTURE_QUALITY = 0x005947d0;
    private static GetTextureQuality? _quality;
    /// <summary>The PS2 files of every installed mod (ModFiles.ps2_files), as paths under efl/x, lower case with '/'.</summary>
    private static HashSet<string> _replaced = [];
    /// <summary>Whether the cluster being loaded on this thread is a PS2 file (null: not looked up yet).</summary>
    [ThreadStatic] private static bool? _ours;
    private static int _kept_full;
    /// <summary>Clusters logged with their decision (the first ones of a session).</summary>
    private const int DEBUG_LINES = 40;

    private static string norm(string path) => path.Replace('\\', '/').ToLowerInvariant();

    /// <summary>
    /// Whether <paramref name="path"/> (as loadCluster gets it) is a PS2 file of a mod: it, or
    /// what follows one of its '/', is a path under efl/x.
    /// </summary>
    private static bool replaced(string path) {
        string p = norm(path);
        if (_replaced.Contains(p)) return true;
        for (int i = p.IndexOf('/'); i >= 0; i = p.IndexOf('/', i + 1))
            if (_replaced.Contains(p[(i + 1)..])) return true;
        return false;
    }

    /// <summary>
    /// The PS2 textures load at their full size at every texture quality: while a PS2 file is
    /// loading, the texture quality reads as TQ_HIGH (no mip level dropped); the game's own files
    /// keep the quality of the graphics options. The PS2 files are those of every installed mod
    /// that demaster.exe built (this one, PS2 interface, PS2 lighting, PS2 maps, PS2
    /// effects, ...), where that mod's file is the one the game gets (a later mod in the load order
    /// wins): ModFiles.ps2_files, read once at start-up, like Fahrenheit's file index.
    /// </summary>
    public static bool texture_quality(ModHost m, string value) {
        _log = m.log;
        (HashSet<string> files, var per_mod) = ModFiles.ps2_files(".phyre");
        _replaced = files;
        _log.Info($"render.texture_quality: {files.Count} PS2 phyre files load at full size ({string.Join(", ", per_mod.Select(p => $"{p.Mod} {p.Count}"))})");
        if (!cluster_hook(m)) return false;
        _quality = render => {
            int q = at<GetTextureQuality>(GET_TEXTURE_QUALITY).chain_from(_quality!).fnptr!(render);
            if (q == 0 || _cluster is not { } path) return q;
            if (_ours is null) {
                _ours = replaced(path);
                if (Interlocked.Increment(ref _kept_full) <= DEBUG_LINES)
                    _log?.Info($"render.texture_quality: '{path}' {(_ours == true ? "at full size (texture quality " + q + " -> 0)" : "kept at texture quality " + q)}");
            }
            return _ours == true ? 0 : q;
        };
        return at<GetTextureQuality>(GET_TEXTURE_QUALITY).hook(m, _quality);
    }
}
