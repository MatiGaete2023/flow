# Procesos de verificación con plazo y diagnóstico; no espera indefinidamente.
function Invoke-CheckedProcess {
    param(
        [Parameter(Mandatory=$true)][string]$FilePath,
        [string[]]$Arguments = @(),
        [Parameter(Mandatory=$true)][string]$Label,
        [ValidateRange(1,1800)][int]$TimeoutSeconds = 180,
        [string]$WorkingDirectory = ''
    )
    Write-Host "INICIO: $Label (plazo $TimeoutSeconds s)"
    $options = @{ FilePath=$FilePath; PassThru=$true; WindowStyle='Hidden' }
    if ($Arguments.Count) { $options.ArgumentList = $Arguments }
    if ($WorkingDirectory) { $options.WorkingDirectory = $WorkingDirectory }
    $process = Start-Process @options
    try {
        # Retener el handle permite consultar ExitCode incluso si termina rápido.
        $null = $process.Handle
        if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
            & taskkill.exe /PID $process.Id /T /F | Out-Null
            throw "Plazo agotado: $Label; se detuvo el proceso de prueba y sus hijos."
        }
        $process.Refresh()
        if ($null -eq $process.ExitCode -or $process.ExitCode -ne 0) {
            throw "Falló ${Label}: código $($process.ExitCode)."
        }
        Write-Host "APROBADO: $Label"
    } finally { $process.Dispose() }
}
