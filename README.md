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

## Build

Requires Rust 1.70+ ([install](https://rustup.rs/)).

```sh
git clone <repo-url> && cd calc
cargo build --release
```

### Install

```sh
cargo install --path crates/calc-tui
```

This installs the `calc` binary to `~/.cargo/bin/`.

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
| `Ctrl+N` | New buffer |

**Navigation:**

| Key | Action |
|-----|--------|
| `Up` / `Down` / `Left` / `Right` | Move cursor |
| `Home` / `End` | Start / end of line |
| `PageUp` / `PageDown` | Scroll 10 lines |
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

### Vim Mode (`--vim`)

Launch with `calc --vim` to enable modal editing.

**Modes:**

| Mode | Description |
|------|-------------|
| Normal | Navigate and edit with vim keys (default on start) |
| Insert | Type text freely (`i`, `a`, `o`, `O` to enter) |
| Visual | Select lines (`v` to enter) |
| Command | Execute commands (`:` to enter) |

**Normal mode — mode transitions:**

| Key | Action |
|-----|--------|
| `i` | Enter Insert mode |
| `a` | Enter Insert mode after cursor |
| `A` | Enter Insert mode at end of line |
| `o` | Open line below and enter Insert |
| `O` | Open line above and enter Insert |
| `v` | Enter Visual (line) mode |
| `:` | Enter Command mode |

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

**Normal mode — undo / redo:**

| Key | Action |
|-----|--------|
| `u` | Undo |
| `Ctrl+R` | Redo |

**Normal mode — editing:**

| Key | Action |
|-----|--------|
| `x` | Delete character under cursor |
| `dd` | Delete current line |
| `yy` | Yank (copy) current line |
| `p` | Paste below |
| `P` | Paste above |

**Insert mode:**

| Key | Action |
|-----|--------|
| `Esc` | Return to Normal mode |
| All other keys | Standard text editing (arrows, Home/End, PgUp/PgDn, Tab, etc.) |

**Visual mode (line-wise):**

| Key | Action |
|-----|--------|
| `j` / `k` | Extend selection down / up |
| `h` / `l` | Move cursor left / right |
| `G` | Extend selection to last line |
| `d` | Delete selected lines |
| `y` | Yank selected lines |
| `Esc` | Cancel selection |

**Command mode:**

| Command | Action |
|---------|--------|
| `:w` | Save (prompts for filename if none set) |
| `:w <file>` | Save to file |
| `:q` | Quit (warns if unsaved changes) |
| `:q!` | Force quit without saving |
| `:wq` / `:x` | Save and quit |
| `Esc` | Cancel command |

### Notes

- The `.calc` extension is added automatically when saving without one.
- Opening a non-existent file (`calc notes.calc`) creates it on first save.
- The left pane is an editor, the right pane shows results in real-time as you type.

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
