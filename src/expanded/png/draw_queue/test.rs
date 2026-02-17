use crate::expanded::png::draw_queue::DrawQueue;

use super::*;

#[test]
fn test_filled_rest_one() {
    let mut queue = DrawQueue::new();

    queue.queue(DrawTask::FilledRect {
        x: 100,
        y: 100,
        width: 100,
        height: 100,
        rgba: (0, 0, 0, 255),
    });

    let trimmed = queue.trim((120, 120), (queue.width() as i32, queue.height() as i32));

    assert_eq!(trimmed.queue.len(), 1);
    if let DrawTask::FilledRect {
        x,
        y,
        width,
        height,
        rgba,
        hatch,
        radius,
    } = &trimmed.queue[0]
    {
        assert_eq!(*x, 0);
        assert_eq!(*y, 0);
        assert_eq!(*width, 80);
        assert_eq!(*height, 80);
        assert_eq!(*rgba, (0, 0, 0, 255));
    } else {
        panic!("Expected a FilledRect task");
    }
}

#[test]
fn test_filled_rest_two() {
    let mut queue = DrawQueue::new();

    queue.queue(DrawTask::FilledRect {
        x: 100,
        y: 100,
        width: 100,
        height: 100,
        rgba: (0, 0, 0, 255),
        hatch: None,
        radius: None,
    });

    let trimmed = queue.trim(
        (120, 120),
        (queue.width() as i32 - 20i32, queue.height() as i32 - 20i32),
    );

    assert_eq!(trimmed.queue.len(), 1);
    if let DrawTask::FilledRect {
        x,
        y,
        width,
        height,
        rgba,
        hatch: _,
        radius: _,
    } = &trimmed.queue[0]
    {
        assert_eq!(*x, 0);
        assert_eq!(*y, 0);
        assert_eq!(*width, 60);
        assert_eq!(*height, 60);
        assert_eq!(*rgba, (0, 0, 0, 255));
    } else {
        panic!("Expected a FilledRect task");
    }
}

#[test]
fn test_hollow_rect_one() {
    let mut queue = DrawQueue::new();

    queue.queue(DrawTask::FilledRect {
        x: 100,
        y: 100,
        width: 100,
        height: 100,
        rgba: (0, 0, 0, 255),
        hatch: None,
        radius: None,
    });

    let trimmed = queue.trim((0, 50), (200, 150));

    assert_eq!(trimmed.queue.len(), 1);
    if let DrawTask::FilledRect {
        x,
        y,
        width,
        height,
        rgba,
        hatch: _,
        radius: _,
    } = &trimmed.queue[0]
    {
        assert_eq!(*x, 100);
        assert_eq!(*y, 50);
        assert_eq!(*width, 100);
        assert_eq!(*height, 50);
        assert_eq!(*rgba, (0, 0, 0, 255));
    } else {
        panic!("Expected a FilledRect task");
    }
}

#[test]
fn test_trim_out_of_bounds_y() {
    let mut queue = DrawQueue::new();

    queue.queue(DrawTask::FilledRect {
        x: 100,
        y: 200,
        width: 100,
        height: 100,
        rgba: (255, 0, 0, 255),
        hatch: None,
        radius: None,
    });

    let trimmed = queue.trim((0, 50), (200, 150));

    assert!(trimmed.queue.is_empty());
}

#[test]
fn test_trim_partial_overlap_y() {
    let mut queue = DrawQueue::new();

    queue.queue(DrawTask::FilledRect {
        x: 50,
        y: 120,
        width: 100,
        height: 50,
        rgba: (0, 255, 0, 255),
        hatch: None,
        radius: None,
    });

    let trimmed = queue.trim((0, 100), (150, 140));

    assert_eq!(trimmed.queue.len(), 1);
    if let DrawTask::FilledRect {
        x,
        y,
        width,
        height,
        rgba,
        hatch: _,
        radius: _,
    } = &trimmed.queue[0]
    {
        assert_eq!(*x, 50);
        assert_eq!(*y, 20);
        assert_eq!(*width, 100);
        assert_eq!(*height, 20);
        assert_eq!(*rgba, (0, 255, 0, 255));
    } else {
        panic!("Expected a FilledRect task");
    }
}

#[test]
fn test_trim_text_y() {
    let mut queue = DrawQueue::new();

    queue.queue(DrawTask::Text {
        x: 10,
        y: 60,
        scale: 12.0,
        rgba: (0, 0, 255, 255),
        text: String::from("Hello"),
        width: 50,
        height: 10,
    });

    let trimmed = queue.trim((0, 50), (60, 70));

    assert_eq!(trimmed.queue.len(), 1);
    if let DrawTask::Text {
        x,
        y,
        scale,
        rgba,
        text,
        width,
        height,
    } = &trimmed.queue[0]
    {
        assert_eq!(*x, 10);
        assert_eq!(*y, 10);
        assert_eq!(*scale, 12.0);
        assert_eq!(*rgba, (0, 0, 255, 255));
        assert_eq!(text, "Hello");
        assert_eq!(*width, 50);
        assert_eq!(*height, 10);
    } else {
        panic!("Expected a Text task");
    }
}

#[test]
fn test_trim_line_y() {
    let mut queue = DrawQueue::new();

    queue.queue(DrawTask::Line {
        start: (10.0, 40.0),
        end: (50.0, 120.0),
        rgba: (255, 255, 0, 255),
    });

    let trimmed = queue.trim((0, 50), (50, 100));

    assert_eq!(trimmed.queue.len(), 1);
    if let DrawTask::Line { start, end, rgba } = &trimmed.queue[0] {
        assert_eq!(*start, (10.0, 0.0));
        assert_eq!(*end, (50.0, 50.0));
        assert_eq!(*rgba, (255, 255, 0, 255));
    } else {
        panic!("Expected a Line task");
    }
}

#[test]
fn test_trim_copy_y() {
    let mut inner_queue = DrawQueue::new();
    inner_queue.queue(DrawTask::FilledRect {
        x: 10,
        y: 10,
        width: 50,
        height: 50,
        rgba: (128, 128, 128, 255),
    });

    let mut queue = DrawQueue::new();
    queue.queue(DrawTask::Copy {
        draw_queue: inner_queue,
        x: 0,
        y: 20,
    });

    let trimmed = queue.trim((0, 15), (60, 40));

    assert_eq!(trimmed.queue.len(), 1);
    if let DrawTask::Copy { draw_queue, x, y } = &trimmed.queue[0] {
        assert_eq!(*x, 0);
        assert_eq!(*y, 5);
        assert_eq!(draw_queue.queue.len(), 1);
    } else {
        panic!("Expected a Copy task");
    }
}

#[test]
fn test_trim_x() {
    let mut queue = DrawQueue::new();

    queue.queue(DrawTask::FilledRect {
        x: 100,
        y: 100,
        width: 100,
        height: 100,
        rgba: (0, 0, 0, 255),
        hatch: None,
        radius: None,
    });

    let trimmed = queue.trim((50, 0), (150, 200));

    assert_eq!(trimmed.queue.len(), 1);
    if let DrawTask::FilledRect {
        x,
        y,
        width,
        height,
        rgba,
        hatch: _,
        radius: _,
    } = &trimmed.queue[0]
    {
        assert_eq!(*x, 50);
        assert_eq!(*y, 100);
        assert_eq!(*width, 50);
        assert_eq!(*height, 100);
        assert_eq!(*rgba, (0, 0, 0, 255));
    } else {
        panic!("Expected a FilledRect task");
    }
}

#[test]
fn test_trim_out_of_bounds_x() {
    let mut queue = DrawQueue::new();

    queue.queue(DrawTask::FilledRect {
        x: 200,
        y: 100,
        width: 100,
        height: 100,
        rgba: (255, 0, 0, 255),
        hatch: None,
        radius: None,
    });

    let trimmed = queue.trim((50, 0), (150, 200));

    assert!(trimmed.queue.is_empty());
}

#[test]
fn test_trim_partial_overlap_x() {
    let mut queue = DrawQueue::new();

    queue.queue(DrawTask::FilledRect {
        x: 120,
        y: 50,
        width: 50,
        height: 100,
        rgba: (0, 255, 0, 255),
        hatch: None,
        radius: None,
    });

    let trimmed = queue.trim((100, 0), (140, 150));

    assert_eq!(trimmed.queue.len(), 1);
    if let DrawTask::FilledRect {
        x,
        y,
        width,
        height,
        rgba,
        hatch: _,
        radius: _,
    } = &trimmed.queue[0]
    {
        assert_eq!(*x, 20);
        assert_eq!(*y, 50);
        assert_eq!(*width, 20);
        assert_eq!(*height, 100);
        assert_eq!(*rgba, (0, 255, 0, 255));
    } else {
        panic!("Expected a FilledRect task");
    }
}

#[test]
fn test_trim_text_x() {
    let mut queue = DrawQueue::new();

    queue.queue(DrawTask::Text {
        x: 60,
        y: 10,
        scale: 12.0,
        rgba: (0, 0, 255, 255),
        text: String::from("Hello"),
        width: 50,
        height: 10,
    });

    let trimmed = queue.trim((50, 0), (70, 20));

    assert_eq!(trimmed.queue.len(), 0);
}

#[test]
fn test_trim_line_x() {
    let mut queue = DrawQueue::new();

    queue.queue(DrawTask::Line {
        start: (40.0, 10.0),
        end: (120.0, 50.0),
        rgba: (255, 255, 0, 255),
    });

    let trimmed = queue.trim((50, 0), (100, 60));

    assert_eq!(trimmed.queue.len(), 1);
    if let DrawTask::Line { start, end, rgba } = &trimmed.queue[0] {
        assert_eq!(*start, (0.0, 10.0));
        assert_eq!(*end, (50.0, 50.0));
        assert_eq!(*rgba, (255, 255, 0, 255));
    } else {
        panic!("Expected a Line task");
    }
}

#[test]
fn test_trim_copy_x() {
    let mut inner_queue = DrawQueue::new();
    inner_queue.queue(DrawTask::FilledRect {
        x: 10,
        y: 10,
        width: 50,
        height: 50,
        rgba: (128, 128, 128, 255),
    });

    let mut queue = DrawQueue::new();
    queue.queue(DrawTask::Copy {
        draw_queue: inner_queue,
        x: 20,
        y: 0,
    });

    let trimmed = queue.trim((15, 0), (40, 60));

    assert_eq!(trimmed.queue.len(), 1);
    if let DrawTask::Copy { draw_queue, x, y } = &trimmed.queue[0] {
        assert_eq!(*x, 5);
        assert_eq!(*y, 0);
        assert_eq!(draw_queue.queue.len(), 1);
    } else {
        panic!("Expected a Copy task");
    }
}

#[test]
fn test_trim_copy_overgrown_y() {
    let mut inner_queue = DrawQueue::new();

    inner_queue.queue(DrawTask::FilledRect {
        x: 0,
        y: 0,
        width: 50,
        height: 50,
        rgba: (0, 0, 0, 255),
        hatch: None,
        radius: None,
    });
    inner_queue.queue(DrawTask::FilledRect {
        x: 0,
        y: 50,
        width: 50,
        height: 50,
        rgba: (128, 128, 128, 255),
        hatch: None,
        radius: None,
    });
    inner_queue.queue(DrawTask::FilledRect {
        x: 0,
        y: 100,
        width: 50,
        height: 50,
        rgba: (0, 0, 0, 255),
        hatch: None,
        radius: None,
    });
    inner_queue.queue(DrawTask::FilledRect {
        x: 0,
        y: 150,
        width: 50,
        height: 50,
        rgba: (0, 0, 0, 255),
        hatch: None,
        radius: None,
    });
    inner_queue.queue(DrawTask::FilledRect {
        x: 0,
        y: 200,
        width: 50,
        height: 50,
        rgba: (128, 128, 128, 255),
        hatch: None,
        radius: None,
    });

    let mut queue = DrawQueue::new();
    queue.queue(DrawTask::Copy {
        draw_queue: inner_queue,
        x: 0,
        y: 0,
    });

    let trimmed = queue.trim((0, 25), (500, 125));

    assert_eq!(trimmed.queue.len(), 1);

    if let DrawTask::Copy { draw_queue, x, y } = &trimmed.queue[0] {
        assert_eq!(draw_queue.queue.len(), 3);
        assert_eq!(*x, 0);
        assert_eq!(*y, 0);

        if let DrawTask::FilledRect {
            x,
            y,
            width,
            height,
            rgba,
            hatch: _,
            radius: _,
        } = &draw_queue.queue[0]
        {
            assert_eq!(*x, 0);
            assert_eq!(*y, 0);
            assert_eq!(*width, 50);
            assert_eq!(*height, 25);
            assert_eq!(*rgba, (0, 0, 0, 255));
        } else {
            panic!("Expected a FilledRect task for the first item");
        }

        if let DrawTask::FilledRect {
            x,
            y,
            width,
            height,
            rgba,
            hatch: _,
            radius: _,
        } = &draw_queue.queue[1]
        {
            assert_eq!(*x, 0);
            assert_eq!(*y, 25);
            assert_eq!(*width, 50);
            assert_eq!(*height, 50);
            assert_eq!(*rgba, (128, 128, 128, 255));
        } else {
            panic!("Expected a FilledRect task for the second item");
        }

        if let DrawTask::FilledRect {
            x,
            y,
            width,
            height,
            rgba,
            hatch: _,
            radius: _,
        } = &draw_queue.queue[2]
        {
            assert_eq!(*x, 0);
            assert_eq!(*y, 75);
            assert_eq!(*width, 50);
            assert_eq!(*height, 25);
            assert_eq!(*rgba, (0, 0, 0, 255));
        } else {
            panic!("Expected a FilledRect task for the third item");
        }
    } else {
        panic!("Expected a Copy task");
    }
}
