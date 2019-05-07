if (!([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator")) { Start-Process powershell.exe "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`"" -Verb RunAs; exit }


function New-TemporaryDirectory {
    $parent = [System.IO.Path]::GetTempPath()
    [string] $name = [System.Guid]::NewGuid()
    New-Item -ItemType Directory -Path (Join-Path $parent $name)
}

function Find-Golem {
	$productCode = (Get-WmiObject -Class Win32_Property -Property ProductCode -Filter "property='UpgradeCode' and value='{3D57CAE4-2890-4BA4-961E-1BDA1630B605}'").ProductCode

	$paths = @(
  		'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\',
  		'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\'
	)
	if ($productCode) {
		foreach ($path in $paths) {
			$obj = Get-ChildItem -Path "$path" | 
    				Get-ItemProperty |
    				Where-Object -Property PSChildName -eq "$productCode"
		 	if ($obj) {
				return (New-Object psobject -Property @{Location=$obj.InstallLocation;Version=$obj.DisplayVersion})
			}
		}
	}
}

Function Get-Bin-Version([string]$app, [string]$bin) {
	if (Test-Path -Path $bin) {
		$headLine = (&$bin --help | Select -first 1 ).split()
		if ($headLine[0] -eq $app) {
			return $headLine[1]
		}
	}
}

 	
Write-Host "-== GOLEM CLI DEV Update ==-"
$golemVersion = Find-Golem
if (!$golemVersion) {
	Write-Host "Golem installation not found"
	return
}

Write-Host "Found golem version: $($golemVersion.Version)"


$repo = "golemfactory/golem-client"
$name = "golemcli"

$releases = "https://api.github.com/repos/$repo/releases"

Write-Host Determining latest release

[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$tag = (Invoke-WebRequest -Uri $releases -UseBasicParsing | ConvertFrom-Json)[0].tag_name

$currentVersion=Get-Bin-Version $name "$($golemVersion.Location)\$name.exe"
Write-Host current cli version: $currentVersion
Write-Host new cli version: $tag

$q = Read-Host -Prompt 'Continue [yN]'


$zip = "$name-windows-$tag.zip"

$download = "https://github.com/$repo/releases/download/$tag/$zip"
$dir = "$name-$tag"

Write-Host Dowloading latest release
Write-Host "download=$download"
Write-Host "zip=$zip"

$out_zip=New-TemporaryFile | Rename-Item -NewName { $_ -replace 'tmp$', 'zip' } -PassThru

[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
Invoke-WebRequest -Uri $download -Out $out_zip

Write-Host Extracting release files
Expand-Archive $out_zip -Force -DestinationPath $golemVersion.Location


