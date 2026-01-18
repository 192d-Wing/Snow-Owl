@echo off
REM startnet.cmd - WinPE startup script for Snow-Owl
REM This file should be placed in the WinPE image at \Windows\System32\startnet.cmd

echo Snow-Owl Windows Deployment System
echo ===================================
echo.

REM Initialize WinPE
wpeinit

REM Wait for network
echo Waiting for network...
ping -n 10 127.0.0.1 > nul

REM Get network configuration via DHCP
echo Configuring network...
wpeutil InitializeNetwork

REM Get server URL from kernel command line or use default
REM In production, you would pass this via iPXE kernel parameters
set SERVER_URL=http://192.168.100.1:8080
set IMAGE_ID=

REM Parse kernel command line for parameters
REM This is a simplified version - in practice you'd parse from registry or file

REM Check if deployment parameters are provided
if "%IMAGE_ID%"=="" (
    echo No automatic deployment configured.
    echo.
    echo To manually deploy an image, run:
    echo   powershell -ExecutionPolicy Bypass -File X:\Deploy-Windows.ps1 -ServerUrl %SERVER_URL% -ImageId [IMAGE_ID]
    echo.
    echo Opening command prompt...
    cmd.exe
) else (
    echo Starting automatic deployment...
    powershell.exe -ExecutionPolicy Bypass -File X:\Deploy-Windows.ps1 -ServerUrl %SERVER_URL% -ImageId %IMAGE_ID%
)
