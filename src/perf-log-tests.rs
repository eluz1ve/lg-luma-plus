use super::*;

fn sample(gap: f64, render: f64) -> Sample {
    Sample {
        gap,
        compose: 2.,
        render,
        swap: 1.,
        image: 0.,
        redrawn: 0.5,
    }
}

#[test]
fn a_second_of_frames_is_one_line_of_means_and_peaks() {
    let line = line(
        12.04,
        "Settings",
        &[sample(16., 6.), sample(18., 10.), sample(32., 8.)],
        &[],
    );
    assert_eq!(
        line,
        "t=12.0 Settings n=3 gap 22.0/32.0 compose 2.0/2.0 render 8.0/10.0 swap 1.0/1.0 image 0.0 redrawn 0.50 late 1\n"
    );
}

#[test]
fn gpu_times_join_the_line_and_late_frames_missed_a_vsync() {
    let line = line(
        3.,
        "Home",
        &[
            sample(16.7, 6.),
            sample(24., 6.),
            sample(26., 6.),
            sample(50., 6.),
        ],
        &[7., 9., 11.],
    );
    assert!(line.ends_with(" late 2 gpu 9.0/11.0\n"), "{line}");
}

#[test]
fn the_switch_file_may_set_launch_options_to_compare() {
    let directory = std::env::temp_dir().join(format!("luma-perf-value-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    assert_eq!(value(&directory, "msaa="), None);
    std::fs::write(directory.join("perf"), "msaa=4\npartial=0\nmsaa= 0 \n").unwrap();
    assert_eq!(value(&directory, "msaa=").as_deref(), Some("0"));
    assert_eq!(value(&directory, "partial=").as_deref(), Some("0"));
    assert_eq!(value(&directory, "script="), None);
    std::fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn the_log_needs_its_switch_file() {
    let directory = std::env::temp_dir().join(format!("luma-perf-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    assert!(PerfLog::open(&directory).is_none());
    std::fs::write(directory.join("perf"), "").unwrap();
    assert!(PerfLog::open(&directory).is_some());
    std::fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn the_switch_file_may_carry_a_script_to_measure() {
    let directory = std::env::temp_dir().join(format!("luma-perf-script-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    assert!(script(&directory).is_empty());
    std::fs::write(directory.join("perf"), "").unwrap();
    assert!(script(&directory).is_empty());
    std::fs::write(
        directory.join("perf"),
        "note\nscript= home, down ,right,wait\nskip=glass,shadow\n",
    )
    .unwrap();
    assert_eq!(script(&directory), ["home", "down", "right", "wait"]);
    assert_eq!(skip(&directory), ["glass", "shadow"]);
    std::fs::remove_dir_all(&directory).unwrap();
}
