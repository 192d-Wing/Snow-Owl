# Deploy-Windows.ps1
# Windows deployment script for Snow-Owl
# This script runs inside WinPE to deploy a Windows image

param(
    [Parameter(Mandatory=$true)]
    [string]$ServerUrl,

    [Parameter(Mandatory=$true)]
    [string]$ImageId,

    [Parameter(Mandatory=$false)]
    [string]$TargetDisk = "0",

    [Parameter(Mandatory=$false)]
    [string]$DeploymentId
)

$ErrorActionPreference = "Stop"

# Logging function
function Write-Log {
    param([string]$Message, [string]$Level = "INFO")
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $logMessage = "[$timestamp] [$Level] $Message"
    Write-Host $logMessage
    Add-Content -Path "X:\deploy.log" -Value $logMessage
}

# Update deployment status
function Update-DeploymentStatus {
    param([string]$Status, [string]$ErrorMessage = $null)

    if ($DeploymentId) {
        try {
            $body = @{
                status = $Status
            }

            if ($ErrorMessage) {
                $body.error_message = $ErrorMessage
            }

            $json = $body | ConvertTo-Json
            Invoke-RestMethod -Uri "$ServerUrl/api/deployments/$DeploymentId/status" `
                -Method POST `
                -Body $json `
                -ContentType "application/json" `
                -TimeoutSec 10
        } catch {
            Write-Log "Failed to update deployment status: $_" "WARN"
        }
    }
}

try {
    Write-Log "Snow-Owl Windows Deployment Starting"
    Write-Log "Server: $ServerUrl"
    Write-Log "Image ID: $ImageId"
    Write-Log "Target Disk: $TargetDisk"

    Update-DeploymentStatus "downloading"

    # Get image information
    Write-Log "Fetching image information..."
    $image = Invoke-RestMethod -Uri "$ServerUrl/api/images/$ImageId" -Method GET

    if (-not $image.success) {
        throw "Failed to get image information: $($image.error)"
    }

    $imageName = $image.data.name
    $imageType = $image.data.image_type
    Write-Log "Image: $imageName ($imageType)"

    # Download the image
    Write-Log "Downloading image..."
    $imageUrl = "$ServerUrl/images/" + ($image.data.file_path -replace '.*[/\\]', '')
    $localImagePath = "X:\image.$imageType"

    # For large files, we'll use BITS transfer if available in WinPE
    # Otherwise fall back to Invoke-WebRequest
    try {
        Start-BitsTransfer -Source $imageUrl -Destination $localImagePath -DisplayName "Downloading Windows Image"
        Write-Log "Download completed using BITS"
    } catch {
        Write-Log "BITS not available, using WebRequest..."
        Invoke-WebRequest -Uri $imageUrl -OutFile $localImagePath
        Write-Log "Download completed"
    }

    Update-DeploymentStatus "installing"

    # Prepare disk
    Write-Log "Preparing disk $TargetDisk..."
    & diskpart /s X:\diskpart-script.txt

    if ($LASTEXITCODE -ne 0) {
        throw "Diskpart failed with exit code $LASTEXITCODE"
    }

    Write-Log "Disk prepared successfully"

    # Apply image based on type
    Write-Log "Applying image to disk..."

    if ($imageType -eq "wim") {
        # Apply WIM image
        Write-Log "Applying WIM image..."
        $wimInfo = & dism /Get-ImageInfo /ImageFile:$localImagePath

        # Apply to partition (assuming C: is the Windows partition)
        & dism /Apply-Image /ImageFile:$localImagePath /Index:1 /ApplyDir:C:\

        if ($LASTEXITCODE -ne 0) {
            throw "DISM apply failed with exit code $LASTEXITCODE"
        }

        # Install bootloader
        Write-Log "Installing bootloader..."
        & bcdboot C:\Windows /s S:

        if ($LASTEXITCODE -ne 0) {
            throw "BCDBoot failed with exit code $LASTEXITCODE"
        }

    } elseif ($imageType -eq "vhd" -or $imageType -eq "vhdx") {
        # Apply VHD/VHDX
        Write-Log "Applying VHD/VHDX image..."
        & dism /Apply-Image /ImageFile:$localImagePath /Index:1 /ApplyDir:C:\

        if ($LASTEXITCODE -ne 0) {
            throw "DISM apply failed with exit code $LASTEXITCODE"
        }

        # Install bootloader
        Write-Log "Installing bootloader..."
        & bcdboot C:\Windows /s S:

        if ($LASTEXITCODE -ne 0) {
            throw "BCDBoot failed with exit code $LASTEXITCODE"
        }
    }

    Write-Log "Image applied successfully"

    # Cleanup
    Write-Log "Cleaning up..."
    Remove-Item -Path $localImagePath -Force -ErrorAction SilentlyContinue

    Update-DeploymentStatus "completed"
    Write-Log "Deployment completed successfully!" "SUCCESS"

    # Reboot
    Write-Log "Rebooting in 10 seconds..."
    Start-Sleep -Seconds 10
    & wpeutil reboot

} catch {
    $errorMsg = $_.Exception.Message
    Write-Log "Deployment failed: $errorMsg" "ERROR"
    Update-DeploymentStatus "failed" $errorMsg

    Write-Host ""
    Write-Host "Press any key to open a command prompt for troubleshooting..."
    $null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
    & cmd.exe
}
