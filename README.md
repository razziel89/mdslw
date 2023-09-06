## Prepare your markdown for easy diff'ing!

<!-- vim-markdown-toc GFM -->

* [About](#about)
* [Motivation](#motivation)
* [Working principle](#working-principle)
    * [Caveats](#caveats)
* [Command reference](#command-reference)
* [Installation](#installation)
* [Editor Integration](#editor-integration)
    * [VIM and NeoVIM](#vim-and-neovim)
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

For text formatted like this, any diff would only show up for the sentences that
are actually affected.
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
  end-of-sentence markers.
  If such a character is followed by whitespace, it denotes the end of a
  sentence, _unless_ the last word before the character is part of a known set
  of words (by default, those are `e.g.`, `i.e.`, `btw.`, and `cf.`).
* Insert a line break after every character that ends a sentence, but keep
  indents in lists in tact.
* Collapse all consecutive whitespace into a single space.
* Wrap single sentences that are longer than the maximum line width (80
  characters by default) without splitting words, keeping indents in tact.

In contrast to most other tools the author could find, `mdslw` does not parse
the entire document into an internal data structure just to render it back
because that might result in changes in unexpected locations.
Instead, it adjusts only those areas that do contain text that can be wrapped.
That is, `mdslw` never touches any parts of a document that cannot be
line-wrapped automatically.
That includes, for example, code blocks, HTML, and pipe tables.

## Caveats

* The default settings of `mdslw` are strongly geared towards the English
  language, even though it should work for other languages, too.
* Like with any other auto-formatter, you give up some fredom for the benefit of
  automatic handling of certain issues.
* Inline code sections are wrapped like any other text, which may cause issues
  with certain renderers.
* While `mdslw` has been tested with documents containing unicode characters
  such as emojis, that testing has been less than rigorous.
  For example, any emoji is treated as a single character when determining line
  width even though some editors might draw certain emojis wider.
  Any feedback is welcome!
* Since `mdslw` collapses all consecutive whitespace into a single space during
  the line-wrapping process, it does not work well with documents using tabs
  in text.
  A tab, including all whitespace before and after it, will also be replaced by
  a single space.

# Command reference

Call as `mdslw [OPTIONS] [PATHS]...`

A `PATH` can point to a file or a directory.
If it is a file, then it will be auto-formatted irrespective of its extension.
If it is a directory, then `mdslw` will discover all files ending in `.md`
recursively and auto-format those.
If you do not specify any path, then `mdslw` will read from stdin and write to
stdout.

- `--help`:
  Prints the help message.
- `--max-width <MAX_WIDTH>`:
  The maximum line width that is acceptable.
  A value of 0 disables wrapping of long lines altogether.
- `--end-markers <END_MARKERS>`:
  The set of characters that are end of sentence markers, defaults to `?!:.`.
- `--keep-words <KEEP_WORDS>`:
  A space-separated list of words that end in one of `END_MARKERS` but that
  should not be followed by a line break, defaults to:
  `cf. btw. etc. e.g. i.e.`
- `--mode <MODE>`:
  A value of `check` means to exit with an error if the format had to be
  adjusted but not to perform any formatting.
  A value of `format`, the default, means to format the file and exit with
  success.
  A value of `both` means to do both (useful when used as a `pre-commit` hook).
- `--upstream <UPSTREAM>`:
  Specify an upstream auto-formatter (with args) that reads from stdin and
  writes to stdout.
  It will be called before `mdslw` will run and `mdslw` will use its output.
  This is useful if you want to chain multiple tools.
  For example, specify `prettier --parser=markdown` to call `prettier` first.
  The upstream auto-formatter is run in each file's directory if `PATHS` are
  specified.

# Installation

❗There are no releses yet for Apple Silicon.
Any help to get them going would be greatly appreciated.
For now, please install via:
```bash
cargo install --git https://github.com/razziel89/mdslw
```
You can get rust including cargo via [rustup][rustup].

Go to the project's [release page][release-page], select the correct
distribution for your system, and download it.
Rename the downloaded binary to `mdslw` (or `mdslw.exe` on Windows) and move it
to a location that is in your `$PATH` such as `/usr/local/bin` (will be
different on Windows).
Moving it there will likely require `root` permissions, e.g. via `sudo`.
On Unix systems, you also have to make the binary executable via the command
`chmod +x mdslw`, pointing to the actual location of `mdslw`.
From now on, you can simply type `mdslw` in your terminal to use it!

# Editor Integration

Contributions describing integrations with more editors are welcome!

## VIM and NeoVIM

Add the following to your `~/.vimrc` or `init.vim` to have your editor
auto-format every `.md` document before writing it out:

```vim
function MdFormat()
  if executable("mdslw")
    set lazyredraw
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
