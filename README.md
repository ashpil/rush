# rush - the really uninteractive shell
Rush is designed to run POSIX scripts, and run them fast. Think of it as a Rust [dash](https://en.wikipedia.org/wiki/Almquist_shell#dash).

I know that the name is overused, but it's too good to pass up.

![pretty demo](https://raw.githubusercontent.com/ashpil/rush/master/demo.png)

### // TODO: 
- [X] Simple command execution `ls -ltr`
- [X] Pipes `exa | grep cargo`
- [X] Exit status logic `! false && ls || date`
- [ ] Redirection
    - [X] File descriptor to another `ls error 2>&1`
    - [X] To/from file `date > time.txt` `< Cargo.toml wc`
    - [X] Appending `>>`
    - [X] Here-docs `<<`
    - [ ] Raw, non-io file descriptors `4>&7`
- [ ] Async execution `&`
- [ ] Shell builtins
   - [ ] Normal built-ins
      - [ ] `alias` `unalias`
      - [X] `cd`
      - [ ] etc
   - [ ] Special built-ins
      - [X] `exit`
      - [ ] `export`
      - [ ] `exec`
      - [ ] etc
- [ ] Expansions
   - [X] Tilde expansion `ls ~`
   - [ ] Parameter expansion
      - [X] Basic expansion `echo ${var:-other}`
      - [ ] String length `echo ${#var}`
      - [ ] Suffix/prefix removal `echo ${var%%pattern}`
   - [X] Command substitution
   - [ ] Arithmetic expansion
- [X] Variables
- [X] Quotes
- [ ] IFS
- [ ] Functions
- [ ] Control flow `if` `for` `while` `case` etc
- [ ] Expand this to-do list


### Decisions to make

* Should this shell replicate commands that are typically built-in but also have system alternatives?
