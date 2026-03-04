# Calc

Real-time notepad calculator. A math engine in pure Rust (zero dependencies) with a terminal UI and CLI eval mode.

```
  salary = 5000                    = 5000
  tax = 15%                        = 0.15
  net = salary - salary * tax      = 4250
  rent = 1200                      = 1200
  savings = net - rent             = 3050
  5 km in miles                    = 3.10686 mi
  sqrt(144)                        = 12
```

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/albrtbc/calc/main/install.sh | sh
```

Or specify a custom directory:

```sh
INSTALL_DIR=~/.local/bin curl -fsSL https://raw.githubusercontent.com/albrtbc/calc/main/install.sh | sh
```

Supports Linux and macOS (amd64/arm64). Installs to `/usr/local/bin` by default.

### Build from source

Requires Rust 1.70+ ([install](https://rustup.rs/)).

```sh
git clone https://github.com/albrtbc/calc && cd calc
cargo install --path crates/calc-tui
```

## Usage

### TUI Mode

```sh
# Launch with empty buffer (simple mode — default)
calc

# Open a file
calc notes.calc

# Launch with vim-style modal editing
calc --vim

# Both together
calc --vim notes.calc
```

### Simple Mode (default)

**General:**

| Key | Action |
|-----|--------|
| `Ctrl+Q` | Quit (prompts if unsaved changes) |
| `Ctrl+S` | Save (prompts for filename if none set) |
| `Ctrl+N` | New buffer tab |
| `Ctrl+W` | Close current tab |

**Navigation:**

| Key | Action |
|-----|--------|
| `Up` / `Down` / `Left` / `Right` | Move cursor |
| `Home` / `End` | Start / end of line |
| `PageUp` / `PageDown` | Scroll 10 lines |
| `Ctrl+PageDown` / `Ctrl+PageUp` | Next / previous tab |
| `Tab` | Insert 2 spaces |

**Selection:**

| Key | Action |
|-----|--------|
| `Shift+Left` / `Shift+Right` | Select character by character |
| `Shift+Up` / `Shift+Down` | Extend selection by line |
| `Shift+Home` / `Shift+End` | Select to start / end of line |
| `Ctrl+Shift+Left` / `Ctrl+Shift+Right` | Select word by word |
| `Ctrl+Shift+Home` / `Ctrl+Shift+End` | Select to start / end of document |

All Shift combinations extend the same selection. Any non-Shift key clears it.

**Undo / Redo:**

| Key | Action |
|-----|--------|
| `Ctrl+Z` | Undo |
| `Ctrl+Shift+Z` | Redo |

**Editing:**

| Key | Action |
|-----|--------|
| `Ctrl+Delete` | Delete word forward |
| `Ctrl+Backspace` | Delete word backward |

**Clipboard:**

Clipboard operations use the system clipboard (compatible with macOS, Linux X11/Wayland, and WSL2).

| Key | Action |
|-----|--------|
| `Ctrl+C` | Copy current line (or selection) to clipboard |
| `Ctrl+X` | Cut current line (or selection) to clipboard |
| `Ctrl+V` | Paste from system clipboard |
| `Ctrl+D` | Delete current line (copies to clipboard) |
| `Ctrl+Y` | Copy current line's result to clipboard |

### Vim Mode (`--vim`)

Launch with `calc --vim` to enable modal editing.

**Modes:**

| Mode | Description |
|------|-------------|
| Normal | Navigate and edit with vim keys (default on start) |
| Insert | Type text freely (`i`, `a`, `o`, `O` to enter) |
| Visual | Select text (`v` for char-wise, `V` for line-wise) |
| Command | Execute commands (`:` to enter) |

**Normal mode — navigation:**

| Key | Action |
|-----|--------|
| `h` `j` `k` `l` | Move left / down / up / right |
| `0` | Move to start of line |
| `$` | Move to end of line |
| `w` | Move to next word start |
| `e` | Move to next word end |
| `b` | Move to previous word start |
| `gg` | Go to first line |
| `G` | Go to last line |
| `<number>G` | Go to line number (e.g. `15G` jumps to line 15) |
| `gt` / `gT` | Next / previous tab |
| `f<char>` | EasyMotion: jump to any occurrence of `<char>` on screen |

Vertical movement (`j`/`k`) preserves your horizontal cursor position across short or empty lines, matching real vim behavior.

**Normal mode — mode transitions:**

| Key | Action |
|-----|--------|
| `i` | Enter Insert mode |
| `a` | Enter Insert mode after cursor |
| `A` | Enter Insert mode at end of line |
| `o` | Open line below and enter Insert |
| `O` | Open line above and enter Insert |
| `v` | Enter Visual (char) mode |
| `V` | Enter Visual (line) mode |
| `:` | Enter Command mode |

**Normal mode — editing:**

| Key | Action |
|-----|--------|
| `x` / `Delete` | Delete character under cursor |
| `dd` | Delete current line |
| `diw` | Delete inner word |
| `cc` | Change (replace) current line |
| `cw` / `ce` | Change word forward |
| `cb` | Change word backward |
| `ciw` | Change inner word |
| `c$` | Change to end of line |
| `c0` | Change to start of line |
| `C` | Change to end of line |
| `s` | Substitute character (delete + insert) |
| `S` | Substitute line (clear + insert) |
| `yy` | Yank (copy) current line |
| `yr` | Copy current line's result to clipboard |
| `p` | Paste below |
| `P` | Paste above |

**Normal mode — undo / redo:**

| Key | Action |
|-----|--------|
| `u` | Undo |
| `Ctrl+R` | Redo |

**Visual mode:**

| Key | Action |
|-----|--------|
| `j` / `k` | Extend selection down / up |
| `h` / `l` | Move cursor left / right |
| `G` | Extend selection to last line |
| `d` / `Delete` | Delete selected text |
| `y` | Yank selected text |
| `Esc` | Cancel selection |

**Command mode:**

| Command | Action |
|---------|--------|
| `:w` | Save (prompts for filename if none set) |
| `:w <file>` | Save to file |
| `:q` | Quit (warns if unsaved changes) |
| `:q!` | Force quit without saving |
| `:wq` / `:x` | Save and quit |
| `:tabnew` | New buffer tab |
| `:tabn` | Next tab |
| `:tabp` | Previous tab |
| `Esc` | Cancel command |

### Mouse

| Action | Effect |
|--------|--------|
| Click on editor | Move cursor to position |
| Click on tab bar | Switch to tab |
| Click on result | Copy result to clipboard |
| Drag | Select text (enters Visual mode in vim) |
| Double-click | Select word under cursor |
| Scroll wheel | Scroll 3 lines up / down |

### Cursorline

The current line is highlighted with a subtle background color across both the editor and results panels, making it easy to identify which result belongs to the line you're editing.

### Notes

- The `.calc` extension is added automatically when saving without one.
- Opening a non-existent file (`calc notes.calc`) creates it on first save.
- The left pane is an editor, the right pane shows results in real-time as you type.
- Errors are isolated per line — a syntax error on one line doesn't affect others.
- Decimal numbers can use either `.` or `,` as separator (`3.5` and `3,5` both work).

### CLI Eval

```sh
# Single expression
calc eval "2 + 2"
# => 4

# Multiple lines
calc eval "x = 10" "x * 2 + 3"
# => 10
# => 23
```

## Language Reference

### Arithmetic

```
2 + 3          # 5
10 - 4         # 6
3 * 7          # 21
20 / 4         # 5
17 % 5         # 2  (modulo)
2 ^ 10         # 1024
-(3 + 4)       # -7
```

### Variables

```
x = 42
y = x * 2       # 84
total = x + y    # 126
```

**Tuple assignment** — multiple variables in one line:

```
(a, b, c) = (10, 20, 30)
a + b + c        # 60
```

### Previous Results

```
100              # 100
ans * 2          # 200  (ans = last result)
_ + 50           # 250  (_ = alias for ans)
ans1 + ans2      # 300  (reference by line number)
```

### Comments and Labels

```
# This is a comment
// This is also a comment

Budget:
rent = 1500
food = 400
```

### Number Formats

```
255              # decimal
0xFF             # hex (255)
0b11111111       # binary (255)
0o377            # octal (255)
1.5e3            # scientific (1500)
1_000_000        # underscores as separators
3,5              # decimal comma (3.5)
```

### Percentages

```
50%              # 0.5
20% of 150       # 30
200 + 15%        # 230
200 - 10%        # 180
```

### Unit Conversions

Use `in`, `to`, or `as` to convert between units:

```
5 km in miles        # 3.10686 mi
100 lb to kg         # 45.3592 kg
72 F in C            # 22.2222 C
1 GB in MB           # 1024 MB
2.5 h in min         # 150 min
```

**Supported units:**

| Category | Units |
|----------|-------|
| Length | m, km, cm, mm, in, ft, yd, mi |
| Mass | kg, g, mg, lb, oz |
| Temperature | C, F, K |
| Data | B, KB, MB, GB, TB |
| Time | s, min, h, d |

**Custom unit conversions** — define variables as rates and convert between them:

```
(euro, dollar, yen) = (1, 0.83, 182.87)
100 euro in dollar       # 83
50 dollar to yen         # 11017.77
1 euro to yen            # 182.87
```

The first value is the base (1). Other values are rates relative to the base: "1 euro = 0.83 dollars = 182.87 yen". Variables defined individually also work as custom units.

### Built-in Functions

```
sqrt(16)             # 4
cbrt(27)             # 3
abs(-5)              # 5
round(3.7)           # 4
floor(3.7)           # 3
ceil(3.2)            # 4
trunc(3.9)           # 3

sin(pi / 2)          # 1
cos(0)               # 1
tan(pi / 4)          # 1
asin(1)              # 1.5707963...
acos(0)              # 1.5707963...
atan(1)              # 0.7853981...
atan2(1, 1)          # 0.7853981...

log(100)             # 2  (log base 10)
log(8, 2)            # 3  (log base 2)
log2(8)              # 3
log10(1000)          # 3
ln(e)                # 1
exp(1)               # 2.71828...

pow(2, 10)           # 1024
factorial(6)         # 720
min(3, 1, 4)         # 1
max(3, 1, 4)         # 4
gcd(12, 8)           # 4
lcm(4, 6)            # 12
```

### Constants

| Name | Value |
|------|-------|
| `pi` / `PI` | 3.14159265... |
| `e` / `E` | 2.71828182... |
| `tau` / `TAU` | 6.28318530... |
| `phi` / `PHI` | 1.61803398... |

## Project Structure

```
calc/
├── crates/
│   ├── calc-core/    # Math engine (pure Rust, zero deps)
│   └── calc-tui/     # Terminal UI (ratatui + crossterm)
```

## Running Tests

```sh
cargo test
```

## License

MIT
