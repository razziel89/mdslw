## Prepare your markdown for easy diff'ing!

<!-- vim-markdown-toc GFM -->

- [About](#about)
- [Motivation](#motivation)
- [Pronunciation](#pronunciation)
- [Working Principle](#working-principle)
  - [Caveats](#caveats)
  - [About Markdown Extensions](#about-markdown-extensions)
- [Command Reference](#command-reference)
  - [Command Line Arguments](#command-line-arguments)
  - [Automatic File Discovery](#automatic-file-discovery)
  - [Environment Variables](#environment-variables)
  - [Config Files](#config-files)
    - [Per-File Configuration](#per-file-configuration)
- [Installation](#installation)
  - [Building From Source](#building-from-source)
- [Editor Integration](#editor-integration)
  - [neovim](#neovim)
  - [vim](#vim)
  - [VS Code](#vs-code)
- [Tips And Tricks](#tips-and-tricks)
  - [Non-Breaking Spaces](#non-breaking-spaces)
  - [Disabling Auto-Formatting](#disabling-auto-formatting)
- [How To Contribute](#how-to-contribute)
- [Licence](#licence)

<!-- vim-markdown-toc -->

# About

This is `mdslw`, the MarkDown Sentence Line Wrapper, an auto-formatter that
prepares your markdown for easy diff'ing.

# Motivation

Markdown documents are written for different purposes.
Some of them are meant to be read in plain text, while others are first rendered
and then presented to the reader.
In the latter case, the documents are often kept in version control and edited
with the same workflows as other code.

When editing source code, software developers do not want changes in one
location to show up as changes in unrelated locations.
Now imagine a markdown document like this:

```markdown
# Lorem Ipsum

Lorem ipsum dolor sit amet. Consectetur adipiscing elit. Sed do eiusmod tempor
incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam.
```

Adding the new sentence `Excepteur sint occaecat cupidatat non proident.` after
the second one and re-arranging the text as a block would result in a diff view
like this that shows changes in several lines:

```diff
3,4c3,5
< Lorem ipsum dolor sit amet. Consectetur adipiscing elit. Sed do eiusmod tempor
< incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam.
---
> Lorem ipsum dolor sit amet. Consectetur adipiscing elit. Excepteur sint occaecat
> cupidatat non proident. Sed do eiusmod tempor incididunt ut labore et dolore
> magna aliqua. Ut enim ad minim veniam.
```

Now imagine the original text had a line break after every sentence, i.e. it had
looked like this:

```markdown
# Lorem Ipsum

Lorem ipsum dolor sit amet.
Consectetur adipiscing elit.
Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.
Ut enim ad minim veniam.
```

For text formatted like this, a diff would only show up for the sentences that
are actually affected, simplifying the review process:

```diff
4a5
> Excepteur sint occaecat cupidatat non proident.
```

Most rendering engines treat a single linebreak like a single space.
Thus, both documents would be identical when presented to the reader even though
the latter is significantly nicer to keep up to date with version control.
The tool `mdslw` aims to auto-format markdown documents in exactly this way.

# Pronunciation

If you are wondering how to pronounce `mdslw`, you can either say each letter
individually or pronounce it like mud-slaw (`mʌd-slɔ`).

# Working Principle

The tool `mdslw` operates according to a very simple process that can be
described as follows:

- Parse the document and determine areas in the document that contain text.
  Only process those.
- There exists a limited number of characters (`.!?:` by default) that serve as
  end-of-sentence markers if they occur alone.
  If such a character is followed by whitespace, it denotes the end of a
  sentence, _unless_ the last word before the character is part of a known set
  of words, matched case-insensitively by default.
  Those words can be taken from an included list for a specific language and
  also specified directly.
- Insert a line break after every character that ends a sentence, but keep
  indents in lists and enumerations in tact.
- Collapse all consecutive whitespace into a single space.
  While doing so, preserve both [non-breaking spaces] and linebreaks that are
  preceded by [non-breaking spaces].
- Before line wrapping, replace all spaces in link texts by
  [non-breaking spaces].
- Wrap lines that are longer than the maximum line width (80 characters by
  default) without splitting words or splitting at [non-breaking spaces] while
  also keeping indents in tact.

In contrast to most other tools the author could find, `mdslw` does not parse
the entire document into an internal data structure just to render it back
because that might result in changes in unexpected locations.
Instead, it adjusts only those areas that do contain text that can be wrapped.
That is, `mdslw` never touches any parts of a document that cannot be
line-wrapped automatically.
That includes, for example, code blocks, HTML blocks, and pipe tables.

## Caveats

- The default settings of `mdslw` are strongly geared towards the English
  language, even though it works for other languages, too.
- Like with any other auto-formatter, you give up some freedom for the benefit
  of automatic handling of certain issues.
- Inline code sections are wrapped like any other text, which may cause issues
  with certain renderers.
- While `mdslw` has been tested with documents containing unicode characters
  such as emojis, the outcome can still be unexpected.
  For example, any emoji is treated as a single character when determining line
  width even though some editors might draw certain emojis wider.
  Any feedback is welcome!
- Since `mdslw` collapses all consecutive whitespace into a single space during
  the line-wrapping process, it does not work well with documents using tabs in
  text.
  A tab, including all whitespace before and after it, will also be replaced by
  a single space.
  Use the `keep-linebreaks` feature and prefix linebreaks by
  [non-breaking spaces] to influence this behaviour.
- There are flavours of markdown that define additional markup syntax that
  `mdslw` cannot recognise but instead detects as text.
  Consequently, `mdslw` might cause formatting changes that causes such special
  syntax to be lost.
  You can use [non-breaking spaces] to work around that.
- Some line breaks added by `mdslw` might not be considered nice looking.
  Use [non-breaking spaces] instead of normal ones to prevent a line break at a
  position.

## About Markdown Extensions

There are quite a lot of markdown extensions out there.
It is not possible for `mdslw` to support all of them.
Instead, `mdslw` aims at supporting CommonMark as well as _some_ extensions used
by its users.
A new extension can be supported if supporting it does not negatively impact
CommonMark support and if support can be added relatively easily.
Please feel free to suggest support for a new extension as a
[contribution](#how-to-contribute).

# Command Reference

Call as:

```bash
mdslw [OPTIONS] [PATHS]...
```

A `PATH` can point to a file or a directory.
If it is a file, then it will be auto-formatted irrespective of its extension.
If it is a directory, then `mdslw` will discover all files ending in `.md`
recursively and auto-format those.
If you do not specify any path, then `mdslw` will read from stdin and write to
stdout.

The following is a list of all supported
[command line arguments](#command-line-arguments).
Note that you can also configure `mdslw` via
[environment variables](#environment-variables) or
[config files](#config-files).
Values are resolved in the following order:

- Defaults
- Config files
- Environment variables
- Command line arguments

## Command Line Arguments

- `--help`:
  Print the help message.
- `--version`:
  Print the tool's version number.
- `--max-width <MAX_WIDTH>`:
  The maximum line width that is acceptable.
  A value of 0 disables wrapping of long lines altogether.
  The default value is 80.
- `--end-markers <END_MARKERS>`:
  The set of characters that are end of sentence markers, defaults to `?!:.`.
- `--mode <MODE>`:
  A value of `check` means to exit with an error if the format had to be
  adjusted but not to perform any formatting.
  A value of `format`, the default, means to format the file and exit with
  success.
  A value of `both` means to do both (useful when used as a `pre-commit` hook).
- `--lang <LANG>`:
  A space-separated list of languages whose suppression words as specified by
  unicode should be taken into account.
  See [here][unicode] for all languages.
  Currently supported are `en`, `de`, `es`, `fr`, and `it`.
  Use `none` to disable.
  Use `ac` (the default) for "author's choice", a list for the English language
  defined and curated by this tool's author.
- `--suppressions <SUPPRESSIONS>`:
  A space-separated list of words that end in one of `END_MARKERS` but that
  should not be followed by a line break.
  This is in addition to what is specified via `--lang`.
  Defaults to the empty string.
- `--ignores <IGNORES>`:
  Space-separated list of words that end in one of `END_MARKERS` and that should
  be removed from the list of suppressions.
  Defaults to the empty string.
- `--upstream-command <UPSTREAM_COMMAND>`:
  Specify an upstream auto-formatter that reads from stdin and writes to stdout.
  It will be called before `mdslw` will run.
  This is useful if you want to chain multiple tools.
  Specify the command that will be executed.
  For example, specify `prettier` to call `prettier` first.
  The upstream auto-formatter runs in each file's directory if `PATHS` are
  specified
- `--upstream <UPSTREAM>`:
  Specify the arguments for the upstream auto-formatter.
  If `--upstream-cmd` is not set, the first word will be used as command.
  For example, with `--upstream-cmd="prettier"`, use
  `--upstream="--parser=markdown"` to enable markdown parsing.
- `--upstream-separator <UPSTREAM_SEPARATOR>`:
  Specify a string that will be used to separate the value passed to
  `--upstream` into words.
  If empty, splitting is based on whitespace.
- `--upstream <UPSTREAM>`:
  Specify an upstream auto-formatter (with args) that reads from stdin and
  writes to stdout.
  It will be called before `mdslw` will run and `mdslw` will use its output.
  This is useful if you want to chain multiple tools.
  For example, specify `prettier --parser=markdown` to call `prettier` first.
  The upstream auto-formatter is run in each file's directory if `PATHS` are
  specified.
- `--case <CASE>`:
  How to handle the case of provided suppression words, both via `--lang` and
  `--suppressions`.
  A value of `ignore`, the default, means to match case-insensitively while a
  value of `keep` means to match case-sensitively.
- `--extension <EXTENSION>`:
  The file extension used to find markdown files when a `PATH` is a directory,
  defaults to `.md`.
- `--link-actions <LINK_ACTIONS>`:
  Link actions to perform.
  Possible values:
  - `outsource-inline`:
    Replace all inline links by named links using a link definition, i.e.
    `[link](url)` becomes `[link][def]` and `[def]: url`.
    All new link definitions will be added at the end of the document.
    Existing link definitions will be reused.
    Link definitions in block quotes will be put at the end of the block quote
    if `--format-block-quotes` is set.
  - `collate-defs`:
    Gather all link definitions, i.e. `[link name]: url`, in a block at the end
    of the document in alphabetical order, sorted case-insensitively.
    Links can be defined as belonging to a category called `CATEGORY_NAME` with
    the comment `<!-- link-category: CATEGORY_NAME -->`.
    Each link definition following such a comment will be considered as part of
    the specified category.
    Link definitions will be sorted per category and categories will also be
    sorted by name.
  - `both`:
    Activate both `outsource-inline` and `collate-defs`.
  Omit this flag to disable all link actions (the default).
- `--keep-whitespace <KEEP_WHITESPACE>`:
  Whitespace preservation options.
  Possible values:
  - `in-links`:
    Do not replace spaces in link texts by [non-breaking spaces].
  - `linebreaks`:
    Do not remove existing linebreaks during the line-wrapping process.
  - `both`:
    Enable both `in-links` and `linebreaks`.
  Omit this flag to disable all whitespace preservation (the default).
- `--format-block-quotes`:
  Format text in block quotes.
  By default, text in block quotes is not formatted.
- `--features <FEATURES>`:
  **Deprecated: Use `--link-actions`, `--keep-whitespace`, and
  `--format-block-quotes` instead.**
  Comma-separated list of optional features to enable or disable.
  This flag is kept for backward compatibility.
  Currently, the following are supported (the opposite setting is the default in
  each case):
  - `keep-spaces-in-links`:
    Do not replace spaces in link texts by [non-breaking spaces].
  - `keep-linebreaks`:
    Do not remove existing linebreaks during the line-wrapping process.
  - `format-block-quotes`:
    Format text in block quotes.
  - `collate-link-defs`:
    Gather all link definitions, i.e. `[link name]: url`, in a block at the end
    of the document in alphabetical order, sorted case-insensitively.
    Links can be defined as belonging to a category called `CATEGORY_NAME` with
    the comment `<!-- link-category: CATEGORY_NAME -->`.
    Each link definition following such a comment will be considered as part of
    the specified category.
    Link definitions will be sorted per category and categories will also be
    sorted by name.
  - `outsource-inline-links`:
    Replace all inline links by named links using a link definition, i.e.
    `[link](url)` becomes `[link][def]` and `[def]: url`.
    All new link definitions will be added at the end of the document.
    Existing link definitions will be reused.
    Link definitions in block quotes will be put at the end of the block quote
    if `format-block-quotes` is set.
- `--completion <COMPLETION>`:
  Output shell completion file for the given shell to stdout and exit.
  The following shells are supported:
  bash, elvish, fish, powershell, zsh.
- `--jobs <JOBS>`:
  Specify the number of threads to use for processing files from disk in
  parallel.
  Defaults to the number of logical processors.
- `--report <REPORT>`:
  What to report to stdout, ignored when reading from stdin:
  - `none`, the default:
    Report nothing but be silent instead, which is useful in scripts.
  - `changed`:
    Output the names of files that were changed, which is useful for downstream
    processing with tools such as `xargs`.
  - `state`:
    Output `<state>:<filename>` where `<state>` is `U` for "unchanged" or `C`
    for "changed", which is useful for downstream filtering with tools such as
    `grep`.
  - `diff-myers`:
    Output a unified diff based on the [myers algorithm].
    Pipe the output to tools such as [bat], [delta], or [diff-so-fancy] to get
    syntax highlighting.
    You can use the `--diff-pager` setting to define such a pager.
  - `diff-patience`:
    Output a unified diff based on the [patience algorithm].
    See `diff-myers` for useful downstream tools.
  - `diff-lcs`:
    Output a unified diff based on the [lcs algorithm].
    See `diff-myers` for useful downstream tools.
- `--diff-pager <DIFF_PAGER>`:
  Specify a downstream pager for diffs (with args) that reads diffs from stdin.
  This is useful if you want to display a diff nicely.
  For example, specify `delta --side-by-side` to get a side-by-side view.
  This flag is ignored unless a diff-type report has been requested.
- `--stdin-filepath <STDIN_FILEPATH>`:
  The path to the file that is read from stdin.
  This is used to determine relevant config files when reading from stdin and to
  run an upstream formatter.
  Defaults to the current working directory.
- `--default-config`:
  Output the default config file in TOML format to stdout and exit.
- `--verbose`:
  Specify to increase verbosity of log output.
  Specify multiple times to increase even further.

## Automatic File Discovery

This tool uses the [ignore crate] in its default settings to discover files when
given a directory as a `PATH`.
Details about those defaults can be found [here][ignore defaults].
Briefly summarised, the following rules apply when deciding whether a file shall
be ignored:

- Hidden files (starting with `.`) are ignored.
- Files matching patterns specified in a file called `.ignore` are ignored.
  The patterns affect all files in the same directory or child directories.
- If run inside a git repository, files matching patterns specified in a file
  called `.gitignore` are ignored.
  The patterns affect all files in the same directory or child directories.

If you wish to format a file that is being ignored by `mdslw`, then pass it as
an argument directly.
Files passed as arguments are never ignored and will always be processed.

## Environment Variables

Instead of or in addition to configuring `mdslw` via
[command line arguments](#command-line-arguments) or
[config files](#config-files), you can configure it via environment variables.
For any command line option `--some-option=value`, you can instead set an
environment variable `MDSLW_SOME_OPTION=value`.
For example, instead of setting `--end-markers=".?!"`, you could set
`MDSLW_END_MARKERS=".?!"` instead.
When set, the value specified via the environment variable will take precedence
over the default value and a value taken from config files.
When set, a command line argument will take precedence over the environment
variable.
Take a call like this for example:

```bash
export MDSLW_EXTENSION=".markdown"
export MDSLW_MODE=both
mdslw --mode=check .
```

This call will search for files with the extension `.markdown` instead of the
default `.md`.
Furthermore, files will only be checked due to `--mode=check`, even though the
environment variable `MDSLW_MODE=both` has been set.
Defaults will be used for everything else.

## Config Files

Instead of or in addition to configuring `mdslw` via
[command line arguments](#command-line-arguments) or
[environment variables](#environment-variables), you can configure it via config
files.
Such a file has to have the exact name `.mdslw.toml` and affects all files in or
below its own directory.
Multiple config files will be merged.
Options given in config files closer to a markdown file take precedence.

Configuration files are limited to options that influence the formatted result.
They cannot influence how `mdslw` operates.
For example, the option `--mode` cannot be set via config files while
`--max-width` can.
The following example shows all the possible options that can be set via config
files.
Note that all entries are optional in config files, which means that any number
of them may be left out.
The following is a full config file containing all the default values.

<!-- cfg-start -->

```toml
max-width = 80
end-markers = "?!:."
lang = "ac"
suppressions = ""
ignores = ""
upstream-command = ""
upstream = ""
upstream-separator = ""
case = "ignore"
features = ""
format-block-quotes = false
# Optional: link-actions = "both"  # Options: outsource-inline, collate-defs, both
# Optional: keep-whitespace = "both"  # Options: in-links, linebreaks, both
```

<!-- cfg-end -->

When set, the value specified via the config file will take precedence over the
default value.
When set, an environment variable or a command line argument will take
precedence over a value taken from config files.

### Per-File Configuration

You can embed a configuration for `mdslw` inside a markdown file.
That configuration affects only the file it is embedded in.
It will be merged with other config files affecting the markdown file in
question just like other config files.

An embedded configuration needs to reside inside the YAML front matter as part
of a _block scalar string_ associated with the YAML key `mdslw-toml` (see below
for an example).
To get an overview of all the different possibilities for defining multi-line
strings in YAML documents, please see [here][yaml-block-scalars].
The embedded configuration string needs to follow the same format as all other
config files for `mdslw` (see above).

For example, you can embed the default config file into a markdown document as
in the following example.
It is strongly recommended to use the `|` block style indicator without a block
chomping indicator as done in the following example.

```markdown
---
# This is the YAML front matter.
mdslw-toml: |
  max-width = 80
  end-markers = "?!:."
  lang = "ac"
  suppressions = ""
  ignores = ""
  upstream-command = ""
  upstream = ""
  upstream-separator = ""
  case = "ignore"
  features = ""
  format-block-quotes = false
  # Optional: link-actions = "both"
  # Optional: keep-whitespace = "both"
---
The actual markdown document follows.
```

Note that `mdslw` does not feature a full YAML parser because, as of October
2025, there is no suitable library available.
Instead, `mdslw` comes with its own limited YAML parser.
That parser supports only block scalar strings without an indentation indicator.

# Installation

Go to the project's [latest release], select the correct binary for your system,
and download it.
See below for how to select the correct one.
Rename the downloaded binary to `mdslw` (or `mdslw.exe` on Windows) and move it
to a location that is in your `$PATH` such as `/usr/local/bin` (will be
different on Windows).
Moving it there will likely require admin or `root` permissions, e.g. via
`sudo`.
On Unix systems, you also have to make the binary executable via the command
`chmod +x mdslw`, pointing to the actual location of `mdslw`.
From now on, you can simply type `mdslw` in your terminal to use it!

The naming of the release binaries uses the [llvm target triple].
You can also use the following list to pick the correct binary for your machine:

- `mdslw_x86_64-unknown-linux-musl`:
  Linux desktop or laptop using 64-bit x86-compatible CPUs
- `mdslw_armv7-unknown-linux-gnueabihf`:
  RaspberryPi or similar single-board computers using ARMv7-compatible CPUs
- `mdslw_x86_64-pc-windows-gnu.exe`:
  Windows desktop or laptop using 64-bit x86-compatible CPUs
- `mdslw_aarch64-apple-darwin`:
  Mac using M1, M2, or other Mx CPUs based on Apple silicon, i.e. the new ones
  after the [transition from Intel CPUs][apple-architecture-transition-arm]
- `mdslw_x86_64-apple-darwin`:
  Mac using 64-bit x86-compatible CPUs, i.e. the old ones after the
  [transition from the PowerPC architecture][apple-architecture-transition-ppc]

## Building From Source

First, install rust, including `cargo`, via [rustup].
Once you have `cargo`, execute the following command in a terminal:

```bash
cargo install --git https://github.com/razziel89/mdslw --locked
```

# Editor Integration

Contributions describing integrations with more editors are welcome!

## neovim

The recommended way of integrating `mdslw` with neovim is through
[conform.nvim].
Simply install the plugin and modify your `init.vim` like this to add `mdslw` as
a formatter for the markdown file type:

```lua
require("conform").setup({
  formatters_by_ft = {
    markdown = { "mdslw" },
  },
  formatters = {
    mdslw = { prepend_args = { "--stdin-filepath", "$FILENAME" } },
  },
})
```

Alternatively, you can also use the vim-like integration shown below.

## vim

Add the following to your `~/.vimrc` to have your editor auto-format every `.md`
document before writing it out:

```vim
function MdFormat()
  if executable("mdslw")
    set lazyredraw
    " Enter and exit insert mode to keep track
    " of the cursor position, useful when undoing.
    execute "normal! ii\<BS>"
    let cursor_pos = getpos(".")
    %!mdslw --stdin-filepath "%"
    if v:shell_error != 0
      u
    endif
    call setpos('.', cursor_pos)
    set nolazyredraw
  endif
endfunction

autocmd BufWritePre *.md silent! :call MdFormat()
```

## VS Code

Assuming you have `mdslw` installed and in your `PATH`, you can integrate it
with VS Code.
To do so, install the extension [run on save] and add the following snippet to
your `settings.json`:

```json
{
  "emeraldwalk.runonsave": {
    "commands": [
      {
        "match": ".*\\.md$",
        "cmd": "mdslw '${file}'"
      }
    ]
  }
}
```

From now on, every time you save to an existing markdown file, `mdslw` will
auto-format it.
This snippet assumes an empty `settings.json` file.
If yours is not empty, you will have to merge it with the existing one.

# Tips And Tricks

## Non-Breaking Spaces

The following codepoints are recognised as [non-breaking spaces] by default:

- U+00A0
- U+2007
- U+202F
- U+2060
- U+FEFF

How to insert [non-breaking spaces] depends on your operating system as well as
your editor.
The below will cover the non-breaking space U+00A0.

**vim/neovim**

Adding this to your `~/.vimrc` or `init.vim` will let you insert non-breaking
spaces when pressing CTRL+s in insert mode and also show them as `+`:

```vim
" Make it easy to insert non-breaking spaces and show them by default.
set list listchars+=nbsp:+
inoremap <C-s> <C-k>NS
" Alternatively, you can use this if your neovim/vim does not support this
" digraph. Note that your browser might not copy the non-breaking space at the
" end of the following line correctly.
inoremap <C-s>  
```

❗Tips for how to add and show non-breaking spaces in other editors are welcome.

## Disabling Auto-Formatting

You can tell `mdslw` to stop auto-formatting parts of your document.
Everything between the HTML comments `<!-- mdslw-ignore-start -->` and
`<!-- mdslw-ignore-end -->` will not be formatted.
For convenience, `mdslw` also recognises `prettier`'s range ignore directives
`<!-- prettier-ignore-start -->` and `<!-- prettier-ignore-end -->`.

In addition, [non-breaking spaces](#non-breaking-spaces) can be used to prevent
modifications to your documents.
Replacing a space by a non-breaking space prevents `mdslw` from adding a line
break at that position.
Furthermore, preceding a line break by a non-breaking space prevents `mdslw`
from removing the line break.

# How To Contribute

If you have found a bug and want to fix it, please simply go ahead and fork the
repository, fix the bug, and open a pull request to this repository!
Bug fixes are always welcome.

In all other cases, please open an issue on GitHub first to discuss the
contribution.
The feature you would like to introduce might already be in development.
Please also take note of [the intended scope](#about-markdown-extensions) of
`mdslw`.

# Licence

[GPLv3]

If you want to use this piece of software under a different, more permissive
open-source licence, please contact me.
I am very open to discussing this point.

<!-- link-category: dependencies -->

[GPLv3]: ./LICENCE
[ignore crate]: https://docs.rs/ignore/latest/ignore/
[ignore defaults]: https://docs.rs/ignore/latest/ignore/struct.WalkBuilder.html#method.standard_filters

<!-- link-category: diff algorithms -->

[lcs algorithm]: https://docs.rs/similar/latest/similar/algorithms/lcs/index.html
[myers algorithm]: https://docs.rs/similar/latest/similar/algorithms/myers/index.html
[patience algorithm]: https://docs.rs/similar/latest/similar/algorithms/patience/index.html

<!-- link-category: diff pagers -->

[bat]: https://github.com/sharkdp/bat
[delta]: https://github.com/dandavison/delta
[diff-so-fancy]: https://github.com/so-fancy/diff-so-fancy

<!-- link-category: editor integrations -->

[conform.nvim]: https://github.com/stevearc/conform.nvim
[run on save]: https://marketplace.visualstudio.com/items?itemName=emeraldwalk.RunOnSave

<!-- link-category: external docs -->

[non-breaking spaces]: https://en.wikipedia.org/wiki/Non-breaking_space
[unicode]: https://github.com/unicode-org/cldr-json/tree/main/cldr-json/cldr-segments-full/segments
[yaml-block-scalars]: https://yaml-multiline.info/

<!-- link-category: installation -->

[apple-architecture-transition-arm]: https://en.wikipedia.org/wiki/Mac_transition_to_Apple_Silicon
[apple-architecture-transition-ppc]: https://en.wikipedia.org/wiki/Mac_transition_to_Intel_processors
[latest release]: https://github.com/razziel89/mdslw/releases/latest
[llvm target triple]: https://clang.llvm.org/docs/CrossCompilation.html#target-triple
[rustup]: https://rustup.rs/
