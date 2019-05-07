#
# $Server = Read-Host -Prompt 'Input your server  name'
#
#

$productCode = (Get-WmiObject -Class Win32_Property -Property ProductCode -Filter "property='UpgradeCode' and value='{3D57CAE4-2890-4BA4-961E-1BDA1630B605}'").ProductCode

echo "pc=$productCode"
#gwmi -Query "SELECT * FROM Win32_Product WHERE IdentifyingNumber = '$productCode'"

$paths = @(
  'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\',
  'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\'
)
    
foreach($path in $paths){
  $obj = Get-ChildItem -Path "$path" | 
    Get-ItemProperty |
    Where-Object -Property PSChildName -eq "$productCode"

  if ($obj) {

    $obj
    $p = $obj.InstallLocation
    $v = $obj.DisplayVersion
    echo "l=$p, v=$v"
    break
  }

    #| 
    #  Select IdentifyingNumber, DisplayName, Publisher, InstallDate, DisplayVersion
}
