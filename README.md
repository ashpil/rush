# rush - the really uninteractive shell
Rush is designed to run POSIX scripts, and run them fast. Think of it as a Rust [dash](https://en.wikipedia.org/wiki/Almquist_shell#dash).

I know that the name is overused, but it's too good to pass up.

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
   - [X] `cd`
   - [X] `exit`
   - [ ] `exec`
   - [ ] etc
- [ ] Environmental variables
- [ ] Variables
- [ ] Quotes
- [ ] Control flow `if` `for` `while` `case` etc
- [ ] Expand this to-do list
