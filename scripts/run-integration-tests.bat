@echo off
REM IPPAN Integration Test Runner for Windows
REM Comprehensive end-to-end testing of the complete IPPAN system

setlocal enabledelayedexpansion

REM Configuration
set TEST_TIMEOUT=300
set LOG_DIR=logs\integration-tests
set REPORT_DIR=reports\integration-tests

REM Create directories
if not exist "%LOG_DIR%" mkdir "%LOG_DIR%"
if not exist "%REPORT_DIR%" mkdir "%REPORT_DIR%"

REM Function to print status
:print_status
echo [%date% %time%] %~1
goto :eof

REM Function to print success
:print_success
echo [%date% %time%] ✅ %~1
goto :eof

REM Function to print error
:print_error
echo [%date% %time%] ❌ %~1
goto :eof

REM Function to print warning
:print_warning
echo [%date% %time%] ⚠️  %~1
goto :eof

REM Function to run a test suite
:run_test_suite
set test_name=%~1
set test_command=%~2
set log_file=%LOG_DIR%\%test_name%.log
set report_file=%REPORT_DIR%\%test_name%.json

call :print_status "Starting %test_name%..."

REM Run test and capture output
%test_command% > "%log_file%" 2>&1
if %errorlevel% equ 0 (
    call :print_success "%test_name% completed successfully"
    echo {"test": "%test_name%", "status": "passed", "timestamp": "%date% %time%"} > "%report_file%"
    set /a failed_tests+=0
) else (
    call :print_error "%test_name% failed (exit code: %errorlevel%)"
    echo {"test": "%test_name%", "status": "failed", "exit_code": %errorlevel%, "timestamp": "%date% %time%"} > "%report_file%"
    set /a failed_tests+=1
)
goto :eof

REM Function to check prerequisites
:check_prerequisites
call :print_status "Checking prerequisites..."

REM Check if Rust is installed
cargo --version >nul 2>&1
if %errorlevel% neq 0 (
    call :print_error "Rust/Cargo is not installed"
    exit /b 1
)

REM Check if the project builds
call :print_status "Building project..."
cargo build --release
if %errorlevel% neq 0 (
    call :print_error "Project build failed"
    exit /b 1
)

call :print_success "Prerequisites check passed"
goto :eof

REM Function to run all tests
:run_all_tests
set start_time=%time%
set failed_tests=0
set total_tests=0

call :print_status "🚀 Starting IPPAN Integration Test Suite"
call :print_status "Test timeout: %TEST_TIMEOUT%s per suite"
call :print_status "Log directory: %LOG_DIR%"
call :print_status "Report directory: %REPORT_DIR%"

REM Test suites
call :run_test_suite "unit_tests" "cargo test --lib --release"
set /a total_tests+=1

call :run_test_suite "integration_tests" "cargo test --test integration --release"
set /a total_tests+=1

call :run_test_suite "performance_tests" "cargo test --test performance_integration --release"
set /a total_tests+=1

call :run_test_suite "security_tests" "cargo test --test security_integration --release"
set /a total_tests+=1

call :run_test_suite "end_to_end_tests" "cargo test --test end_to_end_integration --release"
set /a total_tests+=1

call :run_test_suite "load_tests" "cargo test --test load_tests --release"
set /a total_tests+=1

call :run_test_suite "stress_tests" "cargo test --test stress_tests --release"
set /a total_tests+=1

REM Calculate results
set end_time=%time%
set /a passed_tests=%total_tests%-%failed_tests%

REM Print summary
echo.
call :print_status "📊 Integration Test Results Summary"
call :print_status "Total tests: %total_tests%"
call :print_status "Passed: %passed_tests%"
call :print_status "Failed: %failed_tests%"

if %failed_tests% equ 0 (
    call :print_success "🎉 All integration tests passed!"
    exit /b 0
) else (
    call :print_error "💥 %failed_tests% test suite(s) failed"
    exit /b 1
)

REM Function to run specific test suite
:run_specific_test
set test_name=%~1

if "%test_name%"=="unit" (
    call :run_test_suite "unit_tests" "cargo test --lib --release"
) else if "%test_name%"=="integration" (
    call :run_test_suite "integration_tests" "cargo test --test integration --release"
) else if "%test_name%"=="performance" (
    call :run_test_suite "performance_tests" "cargo test --test performance_integration --release"
) else if "%test_name%"=="security" (
    call :run_test_suite "security_tests" "cargo test --test security_integration --release"
) else if "%test_name%"=="e2e" (
    call :run_test_suite "end_to_end_tests" "cargo test --test end_to_end_integration --release"
) else if "%test_name%"=="load" (
    call :run_test_suite "load_tests" "cargo test --test load_tests --release"
) else if "%test_name%"=="stress" (
    call :run_test_suite "stress_tests" "cargo test --test stress_tests --release"
) else (
    call :print_error "Unknown test suite: %test_name%"
    call :print_status "Available test suites: unit, integration, performance, security, e2e, load, stress"
    exit /b 1
)
goto :eof

REM Function to generate test report
:generate_report
call :print_status "Generating test report..."

set report_file=%REPORT_DIR%\integration_test_report.html

echo ^<!DOCTYPE html^> > "%report_file%"
echo ^<html^> >> "%report_file%"
echo ^<head^> >> "%report_file%"
echo     ^<title^>IPPAN Integration Test Report^</title^> >> "%report_file%"
echo     ^<style^> >> "%report_file%"
echo         body { font-family: Arial, sans-serif; margin: 20px; } >> "%report_file%"
echo         .header { background-color: #f0f0f0; padding: 20px; border-radius: 5px; } >> "%report_file%"
echo         .test-result { margin: 10px 0; padding: 10px; border-radius: 3px; } >> "%report_file%"
echo         .passed { background-color: #d4edda; border: 1px solid #c3e6cb; } >> "%report_file%"
echo         .failed { background-color: #f8d7da; border: 1px solid #f5c6cb; } >> "%report_file%"
echo         .summary { background-color: #e2e3e5; padding: 15px; border-radius: 5px; margin: 20px 0; } >> "%report_file%"
echo     ^</style^> >> "%report_file%"
echo ^</head^> >> "%report_file%"
echo ^<body^> >> "%report_file%"
echo     ^<div class="header"^> >> "%report_file%"
echo         ^<h1^>IPPAN Integration Test Report^</h1^> >> "%report_file%"
echo         ^<p^>Generated: %date% %time%^</p^> >> "%report_file%"
echo     ^</div^> >> "%report_file%"
echo     ^<div class="summary"^> >> "%report_file%"
echo         ^<h2^>Test Summary^</h2^> >> "%report_file%"
echo         ^<p^>Total Tests: %total_tests%^</p^> >> "%report_file%"
echo         ^<p^>Passed: %passed_tests%^</p^> >> "%report_file%"
echo         ^<p^>Failed: %failed_tests%^</p^> >> "%report_file%"
echo     ^</div^> >> "%report_file%"
echo ^</body^> >> "%report_file%"
echo ^</html^> >> "%report_file%"

call :print_success "Test report generated: %report_file%"
goto :eof

REM Function to show help
:show_help
echo IPPAN Integration Test Runner
echo.
echo Usage: %~nx0 [OPTIONS] [TEST_SUITE]
echo.
echo Options:
echo   -h, --help     Show this help message
echo   -r, --report   Generate HTML test report
echo   -t, --timeout  Set test timeout in seconds (default: 300)
echo.
echo Test Suites:
echo   unit          Run unit tests only
echo   integration   Run integration tests only
echo   performance   Run performance tests only
echo   security      Run security tests only
echo   e2e           Run end-to-end tests only
echo   load          Run load tests only
echo   stress        Run stress tests only
echo   all           Run all test suites (default)
echo.
echo Examples:
echo   %~nx0                    # Run all tests
echo   %~nx0 unit               # Run unit tests only
echo   %~nx0 --report           # Run all tests and generate report
echo   %~nx0 -t 600 performance # Run performance tests with 10min timeout
goto :eof

REM Main function
:main
set test_suite=all
set generate_report_flag=false
set timeout_set=false

REM Parse command line arguments
:parse_args
if "%~1"=="" goto :run_tests
if "%~1"=="-h" goto :show_help
if "%~1"=="--help" goto :show_help
if "%~1"=="-r" (
    set generate_report_flag=true
    shift
    goto :parse_args
)
if "%~1"=="--report" (
    set generate_report_flag=true
    shift
    goto :parse_args
)
if "%~1"=="-t" (
    set TEST_TIMEOUT=%~2
    set timeout_set=true
    shift
    shift
    goto :parse_args
)
if "%~1"=="--timeout" (
    set TEST_TIMEOUT=%~2
    set timeout_set=true
    shift
    shift
    goto :parse_args
)
if "%~1"=="unit" (
    set test_suite=unit
    shift
    goto :parse_args
)
if "%~1"=="integration" (
    set test_suite=integration
    shift
    goto :parse_args
)
if "%~1"=="performance" (
    set test_suite=performance
    shift
    goto :parse_args
)
if "%~1"=="security" (
    set test_suite=security
    shift
    goto :parse_args
)
if "%~1"=="e2e" (
    set test_suite=e2e
    shift
    goto :parse_args
)
if "%~1"=="load" (
    set test_suite=load
    shift
    goto :parse_args
)
if "%~1"=="stress" (
    set test_suite=stress
    shift
    goto :parse_args
)
if "%~1"=="all" (
    set test_suite=all
    shift
    goto :parse_args
)

call :print_error "Unknown option: %~1"
call :show_help
exit /b 1

:run_tests
REM Check prerequisites
call :check_prerequisites

REM Run tests
if "%test_suite%"=="all" (
    call :run_all_tests
    if %errorlevel% equ 0 (
        if "%generate_report_flag%"=="true" call :generate_report
        exit /b 0
    ) else (
        if "%generate_report_flag%"=="true" call :generate_report
        exit /b 1
    )
) else (
    call :run_specific_test "%test_suite%"
    if %errorlevel% equ 0 (
        if "%generate_report_flag%"=="true" call :generate_report
        exit /b 0
    ) else (
        if "%generate_report_flag%"=="true" call :generate_report
        exit /b 1
    )
)

REM Run main function
call :main %*
