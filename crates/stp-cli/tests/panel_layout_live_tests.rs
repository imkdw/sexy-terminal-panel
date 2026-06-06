#![allow(clippy::expect_used)]

use assert_cmd::Command;
use assert_cmd::cargo::cargo_bin;
use tempfile::TempDir;

#[test]
fn panel_creates_left_sidebar_and_right_grid() {
    let temp = TempDir::new().expect("temp dir");
    let workspace = temp.path().join("worktree-sidebar-layout");
    std::fs::create_dir(&workspace).expect("workspace");
    let registry = temp.path().join("registry.json");
    let binary = cargo_bin("stp");
    let socket = format!("stp-cli-sidebar-layout-test-{}", std::process::id());
    let terminal_id = "00000000-0000-0000-0000-000000000107";

    kill_tmux_server(&socket);
    register_detached_terminal(&registry, &workspace, &socket, terminal_id);

    for (layout, capacity) in [("2x2", 4usize), ("3x3", 9usize)] {
        let panel_socket = format!(
            "stp-cli-sidebar-layout-outer-{layout}-{}",
            std::process::id()
        );
        let panel_session = format!("stp-cli-sidebar-layout-{layout}");
        kill_tmux_server(&panel_socket);
        Command::new("tmux")
            .args([
                "-L",
                &panel_socket,
                "new-session",
                "-d",
                "-s",
                &panel_session,
                &format!(
                    "STP_TMUX_SOCKET={} {} panel --registry {} --layout {}",
                    shell_quote(&socket),
                    shell_quote(&binary.display().to_string()),
                    shell_quote(&registry.display().to_string()),
                    shell_quote(layout),
                ),
            ])
            .assert()
            .success();

        let panes = wait_for_layout_panes(&socket, capacity.saturating_add(1));
        let sidebar = panes
            .iter()
            .find(|pane| pane.key == "stp-sidebar")
            .expect("sidebar pane");
        assert_eq!(sidebar.left, 0, "{layout} sidebar must stay leftmost");
        assert_eq!(sidebar.width, 44, "{layout} sidebar width");
        assert_content_panes(layout, terminal_id, capacity, &panes, sidebar);
        assert_sidebar_does_not_wrap(layout, &socket, &sidebar.id);
        kill_tmux_server(&panel_socket);
    }

    kill_tmux_server(&socket);
}

fn assert_content_panes(
    layout: &str,
    terminal_id: &str,
    capacity: usize,
    panes: &[PanelPane],
    sidebar: &PanelPane,
) {
    let mut content = panes
        .iter()
        .filter(|pane| pane.key != "stp-sidebar")
        .collect::<Vec<_>>();
    content.sort_by_key(|pane| (pane.top, pane.left));

    assert_eq!(content.len(), capacity, "{layout} content pane count");
    assert!(
        content
            .iter()
            .all(|pane| pane.left >= sidebar.left.saturating_add(sidebar.width)),
        "{layout} content panes must stay to the right of the sidebar"
    );
    let expected_titles = expected_content_titles(terminal_id, capacity);
    let actual_titles = content
        .iter()
        .map(|pane| pane.key.clone())
        .collect::<Vec<_>>();
    assert_eq!(actual_titles, expected_titles, "{layout} row-major titles");

    let (rows, columns) = layout_dimensions(layout).expect("supported test layout");
    let row_tops = unique_sorted(content.iter().map(|pane| pane.top));
    let column_lefts = unique_sorted(content.iter().map(|pane| pane.left));
    assert_eq!(row_tops.len(), rows, "{layout} right-grid row count");
    assert_eq!(
        column_lefts.len(),
        columns,
        "{layout} right-grid column count"
    );

    let row_heights = row_tops
        .iter()
        .map(|top| {
            content
                .iter()
                .find(|pane| pane.top == *top)
                .expect("row pane")
                .height
        })
        .collect::<Vec<_>>();
    let column_widths = column_lefts
        .iter()
        .map(|left| {
            content
                .iter()
                .find(|pane| pane.left == *left)
                .expect("column pane")
                .width
        })
        .collect::<Vec<_>>();
    assert_spread_at_most_one(&row_heights, layout, "right-grid row heights");
    assert_spread_at_most_one(&column_widths, layout, "right-grid column widths");
}

fn register_detached_terminal(
    registry: &std::path::Path,
    workspace: &std::path::Path,
    socket: &str,
    terminal_id: &str,
) {
    Command::cargo_bin("stp")
        .expect("stp binary")
        .args([
            "terminal",
            "--workspace",
            workspace.to_str().expect("utf8 workspace"),
            "--window-id",
            "00000000-0000-0000-0000-000000000001",
            "--terminal-id",
            terminal_id,
            "--socket",
            socket,
            "--registry",
            registry.to_str().expect("utf8 registry"),
            "--shell",
            "sh",
            "--detach",
        ])
        .assert()
        .success();
}

#[derive(Clone, Debug)]
struct PanelPane {
    id: String,
    key: String,
    left: usize,
    top: usize,
    width: usize,
    height: usize,
}

fn wait_for_layout_panes(socket: &str, expected_count: usize) -> Vec<PanelPane> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let panes = list_layout_panes(socket);
        if panes.len() == expected_count
            && panes.iter().any(|pane| pane.key == "stp-sidebar")
            && panes.iter().all(|pane| !pane.key.is_empty())
        {
            return panes;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for {expected_count} layout panes; got {panes:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn list_layout_panes(socket: &str) -> Vec<PanelPane> {
    let output = Command::new("tmux")
        .args([
            "-L",
            socket,
            "list-panes",
            "-t",
            "stp-panel",
            "-F",
            concat!(
                "#",
                "{pane_id}\t#",
                "{@stp-pane-key}\t#",
                "{pane_left}\t#",
                "{pane_top}\t#",
                "{pane_width}\t#",
                "{pane_height}"
            ),
        ])
        .output()
        .expect("layout panes");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse_panel_pane)
        .collect()
}

fn parse_panel_pane(line: &str) -> Option<PanelPane> {
    let mut parts = line.split('\t');
    let id = parts.next()?.to_owned();
    Some(PanelPane {
        id,
        key: parts.next()?.to_owned(),
        left: parts.next()?.parse().ok()?,
        top: parts.next()?.parse().ok()?,
        width: parts.next()?.parse().ok()?,
        height: parts.next()?.parse().ok()?,
    })
}

fn assert_sidebar_does_not_wrap(layout: &str, socket: &str, sidebar_id: &str) {
    let output = Command::new("tmux")
        .args(["-L", socket, "capture-pane", "-pt", sidebar_id, "-S", "-50"])
        .output()
        .expect("sidebar capture");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.lines().all(|line| line.chars().count() <= 44),
        "{layout} sidebar row wrapped: {text}"
    );
}

fn expected_content_titles(terminal_id: &str, capacity: usize) -> Vec<String> {
    std::iter::once(terminal_id.to_owned())
        .chain((2..=capacity).map(|slot| format!("empty:{slot}")))
        .collect()
}

fn layout_dimensions(layout: &str) -> Option<(usize, usize)> {
    match layout {
        "2x2" => Some((2, 2)),
        "3x3" => Some((3, 3)),
        _ => None,
    }
}

fn unique_sorted(values: impl Iterator<Item = usize>) -> Vec<usize> {
    let mut unique = values.collect::<Vec<_>>();
    unique.sort_unstable();
    unique.dedup();
    unique
}

fn assert_spread_at_most_one(values: &[usize], layout: &str, label: &str) {
    let min = values.iter().min().expect("minimum pane size");
    let max = values.iter().max().expect("maximum pane size");
    assert!(
        max.saturating_sub(*min) <= 1,
        "{layout} {label} must be balanced; got {values:?}"
    );
}

fn kill_tmux_server(socket: &str) {
    let _ = Command::new("tmux")
        .args(["-L", socket, "kill-server"])
        .ok();
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
