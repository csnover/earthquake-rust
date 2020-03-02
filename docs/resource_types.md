# Director OSTypes from disassembly

This is a raw dump of every OSType-looking value discovered in disassembly of
projectors. Sometimes types were renamed without changing their content.
This is reflected in the table.

Some of these may not be real, or may be OS-specific, or may be required only
for authoring and not playback.

| Mac 3  | Win 3  | Mac 4† | Win 4  | Win 5  | Mac 5† |
|--------|--------|--------|--------|--------|--------|
| '0000' |        |        |        |        |        |
| '----' |        |        |        |        |        |
| 'MMdp' |        |        |        |        |        |
| 'MMPB' |        |        |        |        |        |
| 'MMDR' |        |        |        |        |        |
| 'MMXO' |        |        |        |        |        |
| 'm5ax' |        |        |        |        |        |
| 'm5cr' |        |        |        |        |        |
| 'm5di' |        |        |        |        |        |
| 'XCOD' |        | 'XCOD' |        |        | 'XCOD' |
| 'XCMD' |        |        |        |        | 'XCMD' |
| 'RMIN' |        |        |        |        |        |
| 'MMCF' |        | 'MMCF' |        |        | 'MMCF' |
| 'FREF' |        | 'FREF' |        |        | 'FREF' |
| 'VWOV' |        |        |        |        |        |
| 'VWPF' |        |        |        |        |        |
| 'VWPR' |        |        |        |        |        |
| 'BNDL' |        | 'BNDL' |        |        | 'BNDL' |
| 'SIZE' |        | 'SIZE' |        |        | 'SIZE' |
| 'DRVR' |        |        |        |        |        |
| 'icl4' |        | 'icl4' |        |        | 'icl4' |
| 'icl8' |        | 'icl8' |        |        | 'icl8' |
| 'ics#' |        | 'ics#' |        |        | 'ics#' |
| 'ics4' |        | 'ics4' |        |        | 'ics4' |
| 'ics8' |        | 'ics8' |        |        | 'ics8' |
| 'VWC0' |        |        |        |        |        |
| 'VWZP' |        |        |        |        |        |
| 'VWst' |        | 'PJst' |        |        | 'PJst' |
| 'VWAD' |        |        |        |        |        |
| 'errs' |        |        |        |        |        |
| 'XFCN' |        |        |        |        | 'XFCN' |
| 'pref' |        |        |        |        |        |
| 'qtim' |        |        |        |        |        |
| 'time' |        |        |        |        |        |
| 'MPGM' |        |        |        |        |        |
| 'out ' |        |        |        |        |        |
| 'misc' |        |        |        |        |        |
| 'dosc' |        |        |        |        |        |
| 'eval' |        |        |        |        |        |
| 'list' |        |        |        |        |        |
| 'CSND' |        |        |        |        |        |
| 'PDEF' |        |        |        |        |        |
| 'XOBJ' |        |        |        |        |        |
| 'appa' |        |        |        |        |        |
| 'INFO' |        |        |        |        |        |
| 'psn ' |        |        |        |        |        |
| 'aevt' |        |        |        |        |        |
| 'odoc' |        |        |        |        |        |
| 'ERIK' |        |        |        |        |        |
| 'PICS' |        |        |        |        |        |
| 'DRWG' |        |        |        |        |        |
| 'amdr' |        |        |        |        |        |
| 'Aout' |        |        |        |        |        |
| 'Bout' |        |        |        |        |        |
| 'memo' | ?      | ?      | ?      | ?      | ?      |
| 'MIDI' | ?      | ?      | ?      | ?      | ?      |
| 'EMPO'*| ?      | ?      | ?      | ?      | ?      |
| 'TRNS'*| ?      | ?      | ?      | ?      | ?      |
| 'SOND'*| ?      | ?      | ?      | ?      | ?      |
| 'Acas'*| ?      | ?      | ?      | ?      | ?      |
| 'Arez'*| ?      | ?      | ?      | ?      | ?      |
| 'vers' | 'ver '?| 'vers' | 'vers' | 'vers' | 'vers' |
| 'VWCF' | 'vwcf' | 'VWCF' | 'VWCF' | 'VWCF' |        |
| 'VWCR' | 'vwcr' | 'VWCR' | 'VWCR' | 'VWCR' |        |
| 'VWSC' | 'vwsc' |        | 'VWSC' | 'VWSC' |        |
| 'VWtc' | 'vwtc' |        | 'VWtc' | 'VWtc' |        |
| 'VWFM' | 'vwfm' |        | 'VWFM' | 'VWFM' |        |
| 'VWTL' | 'vwtl' |        | 'VWTL' | 'VWTL' |        |
| 'VWLB' | 'vwlb' |        | 'VWLB' | 'VWLB' |        |
| 'VWFI' | 'vwfi' |        | 'VWFI' | 'VWFI' |        |
| 'VWCI' | 'vwci' |        | 'VWCI' | 'VWCI' |        |
| 'snd ' | 'snd ' |        | 'snd ' | 'snd ' |        |
| 'CURS' | 'curs' | 'CURS' | 'CURS' | 'CURS' | 'CURS' |
| 'FORM' | 'FORM' |        | 'FORM' | 'FORM' |        |
| 'AIFF' | 'AIFF' |        | 'AIFF' | 'AIFF' |        |
| 'COMM' | 'COMM' |        | 'COMM' | 'COMM' |        |
| 'VWAC' | 'vwac' |        |        | 'VWAC' |        |
| 'ROWN' |        | 'ROWN' | 'ROWN' | 'ROWN' | 'ROWN' |
| 'ICN#' |        | 'ICN#' | 'ICN#' | 'ICN#' | 'ICN#' |
| 'VWMD' |        |        | 'VWMD' | 'VWMD' |        |
| 'TEXT' |        |        | 'TEXT' | 'TEXT' |        |
| 'CLUT' |        |        | 'CLUT' | 'CLUT' |        |
| 'BITD' |        | 'BITD' | 'BITD' | 'BITD' |        |
| 'PICT' |        |        | 'PICT' | 'PICT' | 'PICT' |
| 'sysv' |        |        | 'sysv' | 'sysv' |        |
| 'qd  ' |        |        | 'qd  ' | 'qd  ' |        |
| 'evnt' |        |        | 'evnt' | 'evnt' |        |
| 'alis' |        |        | 'alis' | 'alis' |        |
| 'fold' |        |        | 'fold' | 'fold' |        |
| 'CODE' |        | 'CODE' | 'CODE' | 'CODE' | 'CODE' |
| 'STR ' |        |        | 'STR ' | 'STR ' | 'STR ' |
| 'STR#' |        | 'STR#' | 'STR#' | 'STR#' | 'STR#' |
| 'WIND' |        | 'WIND' | 'WIND' | 'WIND' | 'WIND' |
| 'CMAP' |        | 'CMAP' | 'CMAP' | 'CMAP' | 'CMAP' |
| 'PAT#' |        | 'PAT#' | 'PAT#' | 'PAT#' | 'PAT#' |
| 'ALRT' |        | 'ALRT' | 'ALRT' | 'ALRT' | 'ALRT' |
| 'dctb' |        |        | 'dctb' | 'dctb' | 'dctb' |
| 'DLOG' |        | 'DLOG' | 'DLOG' | 'DLOG' | 'DLOG' |
| 'DITL' |        | 'DITL' | 'DITL' | 'DITL' | 'DITL' |
| 'ICON' |        |        | 'ICON' | 'ICON' |        |
| 'MDRW' |        |        | 'MDRW' | 'MDRW' |        |
| 'OVWD' |        | 'OVWD' | 'OVWD' |        | 'OVWD' |
| 'MD93' |        |        | 'MD93' |        |        |
| 'APPL' |        |        |        | 'APPL' |        |
| 'Tdta' |        | 'Tdta' |        | 'Tdta' | 'Tdta' |
|        | 'mcnm' |        |        |        |        |
|        | 'mftx' |        |        |        |        |
|        | 'ver.' |        |        |        |        |
|        | 'RMMP' |        | 'RMMP' |        |        |
| 'STXT' | 'stxt' |        | 'STXT' | 'STXT' |        |
|        | 'clut' | 'clut' | 'clut' | 'clut' | 'clut' |
| See [1]| 'dib ' |        | 'DIB ' | 'DIB ' |        |
|        | 'scvw' |        | 'SCVW' | 'SCVW' |        |
|        | 'moov' |        | 'moov' | 'moov' |        |
|        | 'RIFF' |        | 'RIFF' | 'RIFF'‡|        |
|        | 'SSND' |        | 'SSND' | 'SSND' |        |
|        | 'WAVE' |        |        | 'WAVE' |        |
|        | 'fmt ' |        |        | 'fmt ' |        |
|        | 'data' |        |        | 'data' |        |
|        |        | 'PJ93' |        |        | 'PJ95' |
|        |        | 'NFNT' |        |        |        |
|        |        | 'FOND' |        |        |        |
|        |        |        | 'VWXO' |        |        |
|        |        |        | 'ASI ' |        |        |
|        |        |        | 'ASID' |        |        |
|        |        |        | 'CTyp' |        |        |
|        |        | 'SYUT' | 'SYUT' | 'SYUT' | 'SYUT' |
|        |        | 'WDEF' | 'WDEF' | 'WDEF' | 'WDEF' |
|        |        |        | 'free' | 'free' |        |
|        |        |        | 'acur' | 'acur' |        |
|        |        |        | 'VWtk' | 'VWtk' |        |
| 'MooV' |        |        | 'MooV' | 'MooV' |        |
|        |        |        | 'crsr' | 'crsr' |        |
|        |        |        | 'RIFX' | 'RIFX' |        |
|        |        |        | 'junk' | 'junk' |        |
|        | 'cftc' |        | 'mmap' | 'mmap' |        |
|        |        |        | 'PICR' | 'PICR' |        |
|        |        |        | 'RGPT' | 'RGPT' |        |
|        |        |        | 'imap' | 'imap' |        |
|        |        |        | 'mach' | 'mach' |        |
|        |        |        | 'proc' | 'proc' |        |
|        |        |        | 'fpu ' | 'fpu ' |        |
|        |        |        | 'hdwr' | 'hdwr' |        |
|        |        |        | 'scri' | 'scri' |        |
|        |        |        | 'os  ' | 'os  ' |        |
|        |        |        | 'te  ' | 'te  ' |        |
|        |        |        | 'edtn' | 'edtn' |        |
|        |        |        | 'help' | 'help' |        |
|        |        |        | 'MARK' | 'MARK' |        |
|        |        |        | 'pop!' | 'pop!' |        |
|        |        |        | 'kbd ' | 'kbd ' |        |
|        |        |        | 'atlk' | 'atlk' |        |
|        |        |        | 'MV93' | 'MV93' |        |
|        |        |        | 'KEY*' | 'KEY*' |        |
|        |        |        | 'Sord' | 'Sord' |        |
|        |        |        | 'FXmp' | 'FXmp' |        |
|        |        |        | 'Fmap' | 'Fmap' |        |
|        |        |        | 'LDEF' | 'LDEF' |        |
|        |        |        | 'INTL' | 'INTL' |        |
|        |        |        | 'INIT' | 'INIT' |        |
|        |        |        | 'INST' | 'INST' |        |
|        |        |        | 'MBAR' | 'MBAR' |        |
|        |        |        | 'PAT ' | 'PAT ' |        |
|        |        |        | 'pltt' | 'pltt' |        |
|        |        |        | 'ppat' | 'ppat' | 'ppat' |
|        |        |        | 'PACK' | 'PACK' |        |
| 'MOVI' |        |        | 'MOVI' | 'MOVI' |        |
| 'BUTT' |        |        | 'BUTT' | 'BUTT' |        |
|        |        |        | 'SHAP' | 'SHAP' |        |
|        |        |        | 'SCRI' | 'SCRI' |        |
|        |        |        | 'TXTS' | 'TXTS' |        |
|        |        |        | 'SICN' | 'SICN' |        |
|        |        |        | 'CDEF' | 'CDEF' |        |
|        |        |        | 'M!93' | 'M!93' |        |
|        |        |        | 'THUM' | 'THUM' |        |
|        |        |        | 'Lnam' | 'Lnam' |        |
|        |        |        | 'Lval' | 'Lval' |        |
|        |        |        | 'Lscr' | 'Lscr' |        |
|        |        |        | 'cicn' | 'cicn' |        |
|        |        |        | 'MENU' | 'MENU' |        |
|        |        |        | 'MDEF' | 'MDEF' |        |
|        |        |        | 'FONT' | 'FONT' |        |
|        |        |        | 'font' | 'font' |        |
|        |        |        | 'CNTL' | 'CNTL' |        |
|        |        |        | 'CASt' | 'CASt' |        |
|        |        |        | 'CAS*' | 'CAS*' |        | similar to ROWN
|        |        |        | 'Lctx' | 'Lctx' |        |
|        |        |        | 'wctb' | 'wctb' |        |
|        |        |        |        | 'actb' |        |
|        |        |        |        | 'Ttt#' |        |
|        |        |        |        | 'Wpt#' |        |
|        |        |        |        | 'WPRF' |        |
|        |        |        |        | 'CPal' |        |
|        |        |        |        | 'ttxt' |        |
|        |        |        |        | 'CMNU' |        |
|        |        |        |        | 'SCRF' |        |
|        |        |        |        | 'Alrt' |        |
|        |        |        |        | 'PAL ' |        |
|        |        |        |        | 'MMIX' |        |
|        |        |        |        | 'ima4' |        |
|        |        |        |        | 'Fres' |        |
|        |        |        |        | 'HKEY' |        |
|        |        |        |        | 'MMUI' |        |
|        |        |        |        | 'LHID' |        |
|        |        |        |        | 'Num#' |        |
|        |        |        |        | 'MMVW' |        |
|        |        |        |        | 'PATH' |        |
|        |        |        |        | 'RPRF' |        |
|        |        |        |        | 'Spt#' |        |
|        |        |        |        | 'sdiv' |        |
|        |        |        |        | 'cdiv' |        |
|        |        |        |        | 'RCAM' |        |
|        |        |        |        | 'mICN' |        |
|        |        |        |        | 'MCsL' |        |
|        |        |        |        | 'scK#' |        |
|        |        |        |        | 'tile' | 'tile' |
|        |        |        |        | 'till' | 'till' |
|        |        |        |        | 'List' |        |
|        |        |        |        | 'Dict' |        |
|        |        |        |        | 'OLED' |        |
|        |        |        |        | 'RTE0' |        |
|        |        |        |        | 'RTE1' |        |
|        |        |        |        | 'RTE2' |        |
|        |        |        |        | 'RTES' |        |
|        |        |        |        | 'AIFC' |        |
|        |        |        |        | 'MV95' |        |
|        |        |        |        | 'MC95' |        |
|        |        |        |        | 'ralf' |        |
|        |        |        |        | 'ACC#' |        |
|        |        |        |        | 'ccl ' |        |
|        |        |        |        | 'LASO' |        |
|        |        |        |        | 'soun' |        |
|        |        |        |        | 'musi' |        |
|        |        |        |        | 'vide' |        |
|        |        |        |        | 'text' |        |
|        |        |        |        | 'PNTG' |        |
|        |        |        |        | 'RTF ' |        |
|        |        |        |        | 'sfil' |        |
|        |        |        |        | 'XMED' |        |
|        |        |        |        | 'CTrn' |        |
|        |        |        |        | 'Cinf' |        |
|        |        |        |        | 'FSSD' |        |
|        |        |        |        | 'M*95' |        |
|        |        |        |        | 'M!95' |        |
|        |        |        |        |        | 'TMPL' |
|        |        |        |        |        | 'DATA' |
|        |        |        |        |        | 'mhlr' |
|        |        |        |        |        | 'play' |
|        |        |        |        |        | 'thng' |

† - Incomplete  
‡ - .WAV RIFF, not .DXR RIFF

[1] - Mac 3 BITDs were converted to 'DIB ' on Windows
