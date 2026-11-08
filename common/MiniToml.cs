// SPDX-License-Identifier: MIT

namespace Yuta.Demaster;

/// <summary>
/// Reads the subset of TOML that demaster.toml uses: comments, [tables], dotted keys,
/// strings ("basic" and 'literal'), booleans and numbers. Values come back as text
/// ("ps2", "true"), keyed by their full dotted name ("models.monster").
/// The converter (Rust, the full TOML spec) is the authority; anything this reader
/// can't parse is reported as a problem rather than guessed.
/// </summary>
internal static class MiniToml {
    public sealed record Result(Dictionary<string, string> Values, List<string> Problems);

    public static Result parse(string text) {
        Dictionary<string, string> values = new(StringComparer.Ordinal);
        List<string> problems = [];
        string table = "";
        string[] lines = text.Replace("\r\n", "\n").Split('\n');
        for (int n = 0; n < lines.Length; n++) {
            string line = strip_comment(lines[n]).Trim();
            if (line.Length == 0) continue;
            if (line.StartsWith('[')) {
                if (line.StartsWith("[[") || !line.EndsWith(']')) {
                    problems.Add($"line {n + 1}: unsupported table header");
                    continue;
                }
                table = line[1..^1].Trim();
                continue;
            }
            int eq = index_outside_quotes(line, '=');
            if (eq < 0) {
                problems.Add($"line {n + 1}: expected key = value");
                continue;
            }
            string key = string.Join('.', line[..eq].Split('.').Select(k => k.Trim().Trim('"', '\'')));
            string raw = line[(eq + 1)..].Trim();
            string? value = parse_value(raw);
            if (value is null) {
                problems.Add($"line {n + 1}: can't read the value {raw}");
                continue;
            }
            values[table.Length == 0 ? key : $"{table}.{key}"] = value;
        }
        return new Result(values, problems);
    }

    private static string? parse_value(string raw) {
        if (raw.Length >= 2 && raw[0] == '\'' && raw[^1] == '\'') return raw[1..^1];
        if (raw.Length >= 2 && raw[0] == '"' && raw[^1] == '"') return unescape(raw[1..^1]);
        if (raw is "true" or "false") return raw;
        if (double.TryParse(raw.Replace("_", ""), NumberStyles.Float, CultureInfo.InvariantCulture, out _)) return raw;
        return null;
    }

    private static string? unescape(string s) {
        StringBuilder b = new();
        for (int i = 0; i < s.Length; i++) {
            char c = s[i];
            if (c != '\\') { b.Append(c); continue; }
            if (++i >= s.Length) return null;
            switch (s[i]) {
                case '"':  b.Append('"'); break;
                case '\\': b.Append('\\'); break;
                case 'n':  b.Append('\n'); break;
                case 't':  b.Append('\t'); break;
                case 'r':  b.Append('\r'); break;
                case 'b':  b.Append('\b'); break;
                case 'f':  b.Append('\f'); break;
                case 'u' when i + 4 < s.Length: b.Append((char)Convert.ToInt32(s.Substring(i + 1, 4), 16)); i += 4; break;
                default: return null;   // TOML rejects unknown escapes too
            }
        }
        return b.ToString();
    }

    private static string strip_comment(string line) {
        int hash = index_outside_quotes(line, '#');
        return hash < 0 ? line : line[..hash];
    }

    private static int index_outside_quotes(string s, char target) {
        char quote = '\0';
        for (int i = 0; i < s.Length; i++) {
            char c = s[i];
            if (quote != '\0') {
                if (c == '\\' && quote == '"') { i++; continue; }
                if (c == quote) quote = '\0';
            }
            else if (c is '"' or '\'') quote = c;
            else if (c == target) return i;
        }
        return -1;
    }
}
