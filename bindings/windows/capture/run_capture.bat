@echo off
cd /d C:\Users\rise\.RISE\bindings\windows\capture
echo R1-start
if not exist C:\Users\rise\.RISE\target mkdir C:\Users\rise\.RISE\target
capture.exe C:\Users\rise\.RISE\target\frame-001.yuy2
echo R2-captured
type C:\Users\rise\.RISE\target\frame-001.yuy2.meta
C:\Users\rise\.RISE\core\target\debug\rise-core.exe hash C:\Users\rise\.RISE\target\frame-001.yuy2
echo R3-hashed
