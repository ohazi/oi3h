# Oren's i3 Helper

Collection of utilities and additions for the [i3 window manager](https://i3wm.org/). It might also work with [sway](https://swaywm.org/).

## Build
```
$ cargo build --release
$ cp target/release/oi3h /somewhere/in/your/$PATH/
```

## Usage

### Border
`$ oi3h border [...]`

#### Toggle 
Toggle between a list of border styles. List should be provided as arguments after the `toggle` flag. Border styles can be any of `none`, `normal` or `pixel`. The `normal` and `pixel` styles can optionally be followed by a width value (in pixels). Use quotes to ensure that border style and width are part of the same argument.

```
$ oi3h border [--toggle|-t] [list]
$ oi3h border --toggle 'pixel 2' 'normal 2'
$ oi3h border -t normal none
```

Border styles in toggle list should be unique. For example, the following will not work:
```
$ oi3h border -t 'pixel 2' 'pixel 5' 'pixel 10' ; echo $?
Set of border states to toggle should be unique
1
$
```
