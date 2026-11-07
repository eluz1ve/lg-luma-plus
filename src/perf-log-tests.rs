use super::*;

fn sample(gap: f64, render: f64) -> Sample {
    Sample {
        gap,
        compose: 2.,
        render,
        swap: 1.,
        image: 0.,
        redrawn: 0.5,
        laid: false,
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

#[test]
fn a_late_frame_gets_its_own_line_with_its_first_use_work() {
    let directory = std::env::temp_dir().join(format!("luma-perf-late-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("perf"), "").unwrap();
    let mut log = PerfLog::open(&directory).unwrap();
    let frame = |render_ms, image_ms| Frame {
        interval_ms: 0.,
        compose_ms: 1.,
        render_ms,
        swap_ms: 2.,
        redrawn: 1.,
        glyph_ms: 0.,
        vector_ms: 0.,
        image_ms,
        laid: false,
    };
    let start = std::time::Instant::now();
    log.record(start, "Home", &frame(5., 0.));
    log.record(start + Duration::from_millis(17), "Home", &frame(5., 0.));
    log.record(start + Duration::from_millis(57), "Home", &frame(30., 24.));
    drop(log);
    let written = std::fs::read_to_string(directory.join("perf.log")).unwrap();
    let late: Vec<_> = written
        .lines()
        .filter(|line| line.contains(" late frame "))
        .collect();
    assert_eq!(late.len(), 1, "{written}");
    assert!(
        late[0].ends_with(" late frame Home gap 40.0 compose 1.0 render 30.0 swap 2.0 glyph 0.0 vector 0.0 image 24.0"),
        "{}",
        late[0]
    );
    std::fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn a_frame_after_waiting_for_input_is_not_late() {
    let directory = std::env::temp_dir().join(format!("luma-perf-idle-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("perf"), "").unwrap();
    let mut log = PerfLog::open(&directory).unwrap();
    let frame = Frame {
        interval_ms: 0.,
        compose_ms: 1.,
        render_ms: 1.,
        swap_ms: 1.,
        redrawn: 0.1,
        glyph_ms: 0.,
        vector_ms: 0.,
        image_ms: 0.,
        laid: false,
    };
    let start = std::time::Instant::now();
    log.record(start, "Home", &frame);
    log.record(start + Duration::from_millis(17), "Home", &frame);
    // A remote press 200 ms later: the gap was a wait, not a slow frame.
    log.idle();
    log.record(start + Duration::from_millis(217), "Home", &frame);
    log.record(start + Duration::from_millis(234), "Home", &frame);
    log.record(start + Duration::from_millis(1300), "Home", &frame);
    drop(log);
    let written = std::fs::read_to_string(directory.join("perf.log")).unwrap();
    assert!(!written.contains(" late frame "), "{written}");
    assert!(written.contains(" n=2 gap 17.0/17.0 "), "{written}");
    std::fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn frames_that_laid_the_page_down_as_a_picture_are_counted() {
    let laid = Sample {
        laid: true,
        ..sample(16.7, 2.)
    };
    let line = line(
        5.,
        "Home+overlay",
        &[
            sample(16.7, 9.),
            laid,
            Sample {
                laid: true,
                ..sample(16.7, 2.)
            },
        ],
        &[3.],
    );
    assert!(line.ends_with(" late 0 laid 2 gpu 3.0/3.0\n"), "{line}");
}
