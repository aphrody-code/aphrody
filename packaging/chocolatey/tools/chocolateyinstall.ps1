# SPDX-License-Identifier: Apache-2.0
$ErrorActionPreference = 'Stop'
$packageName = 'aphrody'
$toolsDir = "$(Split-Path -parent $MyInvocation.MyCommand.Definition)"
$version = '1.0.0-canary'
$url64 = "https://github.com/aphrody-code/aphrody/releases/download/v$version/aphrody-x86_64-pc-windows-msvc.zip"
$urlArm64 = "https://github.com/aphrody-code/aphrody/releases/download/v$version/aphrody-aarch64-pc-windows-msvc.zip"

$packageArgs = @{
  packageName    = $packageName
  unzipLocation  = $toolsDir
  url64bit       = $url64
  urlArm64       = $urlArm64
  checksum64     = 'PLACEHOLDER-SHA256-AT-RELEASE-TIME'
  checksumType64 = 'sha256'
  checksumArm64  = 'PLACEHOLDER-SHA256-AT-RELEASE-TIME'
  checksumTypeArm64 = 'sha256'
}

Install-ChocolateyZipPackage @packageArgs
