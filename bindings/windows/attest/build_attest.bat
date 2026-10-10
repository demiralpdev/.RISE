@echo off
echo M1-start
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul 2>&1
echo M2-vcvars-ok
cd /d C:\Users\rise\.RISE\bindings\windows\attest
cl /nologo /EHsc /std:c++17 attest.cpp
echo M3-compiled
