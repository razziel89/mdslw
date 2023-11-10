## Prepare your markdown for easy diff'ing!

<!-- vim-markdown-toc GFM -->

* [About](#about)
* [Motivation](#motivation)
* [Working principle](#working-principle)
    * [Caveats](#caveats)
* [Command reference](#command-reference)
    * [Command Line Arguments](#command-line-arguments)
    * [Automatic file discovery](#automatic-file-discovery)
    * [Environment Variables](#environment-variables)
* [Installation](#installation)
    * [Building From Source](#building-from-source)
* [Editor Integration](#editor-integration)
    * [neovim](#neovim)
    * [vim](#vim)
    * [VS Code](#vs-code)
* [Tips and Tricks](#tips-and-tricks)
    * [Non-Breaking Spaces](#non-breaking-spaces)
* [How to contribute](#how-to-contribute)
* [Licence](#licence)

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

# Working principle

The tool `mdslw` operates according to a very simple process that can be
described as follows:

* Parse the document and determine areas in the document that contain text.
  Only process those.
* There exists a limited number of characters (`.!?:` by default) that serve as
  end-of-sentence markers if they occur alone.
  If such a character is followed by whitespace, it denotes the end of a
  sentence, _unless_ the last word before the character is part of a known set
  of words, matched case-insensitively by default.
  Those words can be taken from an included list for a specific language and
  also specified directly.
* Insert a line break after every character that ends a sentence, but keep
  indents in lists and enumerations in tact.
* Collapse all consecutive whitespace into a single space while preserving
  [non-breaking spaces][wiki-nbsp].
* Wrap single sentences that are longer than the maximum line width (80
  characters by default) without splitting words or splitting at
  [non-breaking spaces][wiki-nbsp] while also keeping indents in tact.

In contrast to most other tools the author could find, `mdslw` does not parse
the entire document into an internal data structure just to render it back
because that might result in changes in unexpected locations.
Instead, it adjusts only those areas that do contain text that can be wrapped.
That is, `mdslw` never touches any parts of a document that cannot be
line-wrapped automatically.
That includes, for example, code blocks, HTML blocks, and pipe tables.
Note that some of these settings can be modified via the `--features` flag.

## Caveats

* The default settings of `mdslw` are strongly geared towards the English
  language, even though it works for other languages, too.
* Like with any other auto-formatter, you give up some freedom for the benefit
  of automatic handling of certain issues.
* Inline code sections are wrapped like any other text, which may cause issues
  with certain renderers.
* While `mdslw` has been tested with documents containing unicode characters
  such as emojis, the outcome can still be unexpected.
  For example, any emoji is treated as a single character when determining line
  width even though some editors might draw certain emojis wider.
  Any feedback is welcome!
* Since `mdslw` collapses all consecutive whitespace into a single space during
  the line-wrapping process, it does not work well with documents using tabs
  in text.
  A tab, including all whitespace before and after it, will also be replaced by
  a single space.
* There are flavours of markdown that define additional markup syntax that
  `mdslw` cannot recognise but instead detects as text.
  Consequently, `mdslw` might cause formatting changes that causes such special
  syntax to be lost.
* Some line breaks added by `mdslw` might not be considered nice looking.
  Use a [non-breking space][wiki-nbsp] ` ` instead of a normal space ` ` to
  prevent a line break at a position.

# Command reference

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
[environment variables](#environment-variables).

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
- `--upstream <UPSTREAM>`:
  Specify an upstream auto-formatter (with args) that reads from stdin and
  writes to stdout.
  It will be called before `mdslw` will run and `mdslw` will use its output.
  This is useful if you want to chain multiple tools.
  For example, specify `prettier --parser=markdown` to call `prettier` first.
  The upstream auto-formatter is run in each file's directory if `PATHS` are
  specified.
-  `--case <CASE>`:
   How to handle the case of provided suppression words, both via `--lang` and
   `--suppressions`.
   A value of `ignore`, the default, means to match case-insensitively while a
   value of `keep` means to match case-sensitively.
- `--extension <EXTENSION>`:
  The file extension used to find markdown files when a `PATH` is a directory,
  defaults to `.md`.
- `--features <FEATURES>`:
  Comma-separated list of optional features to enable or disable.
  Currently, the following are supported (the opposite setting is the default in
  each case):
    - `keep-spaces-in-links`:
      Do not replace spaces in link texts by [non-breaking spaces][wiki-nbsp].
    - `keep-inline-html`:
      Prevent modifications of HTML that does not span lines.
    - `keep-footnotes`:
      Prevent modifications to footnotes.
    - `modify-tasklists`:
      Allow modifications to tasklists.
    - `modify-tables`:
      Allow modifications to tables (entire tables, not inside tables).
    - `modify-nbsp`:
      Allow modifications to UTF8 [non-breaking spaces][wiki-nbsp].
      They will be replaced by and treated as regular breaking spaces if set.
    - `breaking-multiple-markers`:
      Insert line breaks after repeated `END_MARKERS`.
      If not set, lines will not break after multiple `END_MARKERS`, e.g. `!?`
      or `...` for the default `END_MARKERS`.
    - `breaking-start-marker`:
      Insert line breaks after a single end marker at the beginning of a line.

## Automatic file discovery

This tool uses the [ignore][ignore] crate in its default settings to discover
files when given a directory as a `PATH`.
Details about those defaults can be found [here][ignore-defaults].
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
[command line arguments](#command-line-arguments), you can configure it via
environment variables.
For any command line option `--some-option=value`, you can instead set an
environment variable `MDSLW_SOME_OPTION=value`.
For example, instead of setting `--end-markers=".?!"`, you could set
`MDSLW_END_MARKERS=".?!"` instead.
When set, the value specified via the environment variable will take precedence
over the default value.
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

# Installation

Go to the project's [release page][release-page], select the correct
distribution for your system, and download it.
Rename the downloaded binary to `mdslw` (or `mdslw.exe` on Windows) and move it
to a location that is in your `$PATH` such as `/usr/local/bin` (will be
different on Windows).
Moving it there will likely require `root` permissions, e.g. via `sudo`.
On Unix systems, you also have to make the binary executable via the command
`chmod +x mdslw`, pointing to the actual location of `mdslw`.
From now on, you can simply type `mdslw` in your terminal to use it!

❗There are no releases yet for Apple Silicon.
Any help to get them going would be greatly appreciated.
For now, please build from source (see below).

## Building From Source

First, install rust, including `cargo`, via [rustup][rustup].
Then, make sure you have `git` installed, too.
Once you have both `cargo` and `git`, execute the following commands in a
terminal:

```bash
git clone https://github.com/razziel89/mdslw
cargo install --locked --path mdslw
```

That way, you will only get the default suppression list.
If you want additional suppression lists such as the ones bundled with the
pre-compiled binaries, you also require the tools `jq`, `make`, and `curl`.
Once you have them installed, run `make -C mdslw build-language-files` before
running the `cargo install` command to retrieve the suppression lists.
The install command will pick them up automatically.

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
    %!mdslw
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
To do so, install the extension [Run on Save][runonsave] and add the following
snippet to your `settings.json`:

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

# Tips and Tricks

## Non-Breaking Spaces

The following codepoints are recognised as non-breking spaces by default:

- U+00A0
- U+2007
- U+202F
- U+2060
- U+FEFF

How to insert a [non-breaking space][wiki-nbsp] depends on your operating
system as well as your editor.
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

❗Tips for how to add and show non-breaking spaces in other editors are
welcome.

# How to contribute

If you have found a bug and want to fix it, please simply go ahead and fork the
repository, fix the bug, and open a pull request to this repository!
Bug fixes are always welcome.

In all other cases, please open an issue on GitHub first to discuss the
contribution.
The feature you would like to introduce might already be in development.

# Licence

[GPLv3](./LICENCE)

If you want to use this piece of software under a different, more permissive
open-source licence, please contact me.
I am very open to discussing this point.

[release-page]: https://github.com/razziel89/mdslw/releases/latest "latest release"
[rustup]: https://rustup.rs/ "rustup"
[unicode]: https://github.com/unicode-org/cldr-json/tree/main/cldr-json/cldr-segments-full/segments
[ignore]: https://docs.rs/ignore/latest/ignore/ "ignore"
[ignore-defaults]: https://docs.rs/ignore/latest/ignore/struct.WalkBuilder.html#method.standard_filters "defaults"
[runonsave]: https://marketplace.visualstudio.com/items?itemName=emeraldwalk.RunOnSave "runonsave"
[conform.nvim]: https://github.com/stevearc/conform.nvim "conform.nvim"
[wiki-nbsp]: https://en.wikipedia.org/wiki/Non-breaking_space "non-breaking spaces"
