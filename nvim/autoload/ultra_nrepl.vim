function! ultra_nrepl#GetCmd() abort
  if !exists('g:ultra_nrepl_cmd')
    let g:ultra_nrepl_cmd = 'ultra_nrepl'
  endif

  return expand(g:ultra_nrepl_cmd)
endfunction

function! ultra_nrepl#FindDef() abort
  if s:ErrorCheck()
    return
  endif

  let symbol = expand('<cword>')
  let fname = expand('%:p')
  let cmd = ultra_nrepl#GetCmd() . ' find_def ' . fname . ' ' . symbol
  let lines = systemlist(cmd)

  for line in lines
    let parts = split(line, ' ')

    if parts[0] ==# 'IS-EMPTY'
      return
    elseif parts[0]  ==# 'LINE'
      let l:linenum = parts[1]
    elseif parts[0] ==# 'COLUMN'
      let l:column = parts[1]
    elseif parts[0] ==# 'FILE'
      let l:file = parts[1]
    elseif parts[0] ==# 'JAR'
      let l:jar = parts[1]
    endif
  endfor

  " echo [l:file, l:linenum, l:column]

  if exists('l:jar')
    let content_cmd = ultra_nrepl#GetCmd() . ' read_jar ' . l:jar . ' ' . l:file
    let contents = systemlist(content_cmd)

    let winview = winsaveview()  " Save the current cursor position

    let l:tmpfname = tempname() . '.clj'

    call writefile(contents, l:tmpfname)

    call s:JumpToLocation(l:tmpfname, l:linenum, l:column)

    return
  endif

  if !exists('l:file')
    return
  endif

  call s:JumpToLocation(l:file, l:linenum, l:column)
endfunction

function! s:Warn(msg) abort
    echohl WarningMsg | echomsg a:msg | echohl NONE
endfunction

function! s:ErrorCheck() abort
    if !executable(ultra_nrepl#GetCmd())
        call s:Warn('No racer executable found in $PATH (' . $PATH . ')')
        return 1
    endif
endfunction

function! s:JumpToLocation(filename, linenum, colnum) abort
    if a:filename ==# ''
        return
    endif

    " Record jump mark
    normal! m`
    if a:filename != expand('%:p')
        try
            exec 'keepjumps e ' . fnameescape(a:filename)
        catch /^Vim\%((\a\+)\)\=:E37/
            " When the buffer is not saved, E37 is thrown.  We can ignore it.
        endtry
    endif
    call cursor(a:linenum, a:colnum)
    " Center definition on screen
    normal! zz
endfunction
