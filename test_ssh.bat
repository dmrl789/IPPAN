@echo off
echo Testing SSH access to both servers...

echo.
echo Testing Server 1 (188.245.97.41)...
ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no ippan@188.245.97.41 "echo Server 1 SSH working"
if %errorlevel% equ 0 (
    echo SUCCESS: Server 1 SSH is working
) else (
    echo FAILED: Server 1 SSH not working
)

echo.
echo Testing Server 2 (135.181.145.174)...
ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no ippan@135.181.145.174 "echo Server 2 SSH working"
if %errorlevel% equ 0 (
    echo SUCCESS: Server 2 SSH is working
) else (
    echo FAILED: Server 2 SSH not working
)

echo.
echo SSH test complete!
pause
