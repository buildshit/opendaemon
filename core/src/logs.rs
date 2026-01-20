use std::collections::{HashMap, VecDeque};
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct LogLine {
    pub timestamp: SystemTime,
    pub content: String,
    pub stream: LogStream,
}

impl LogLine {
    /// Format timestamp as a string for display
    pub fn timestamp_str(&self) -> String {
        match self.timestamp.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(duration) => {
                let secs = duration.as_secs();
                let millis = duration.subsec_millis();
                format!("{}.{:03}", secs, millis)
            }
            Err(_) => "unknown".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum LogStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone)]
pub enum LogLineCount {
    Last(usize),
    All,
}

pub struct CircularBuffer {
    lines: VecDeque<LogLine>,
    max_size: usize,
}

impl CircularBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    pub fn push(&mut self, line: LogLine) {
        if self.lines.len() >= self.max_size {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    pub fn get_lines(&self, count: LogLineCount) -> Vec<LogLine> {
        match count {
            LogLineCount::All => self.lines.iter().cloned().collect(),
            LogLineCount::Last(n) => {
                let start = self.lines.len().saturating_sub(n);
                self.lines.iter().skip(start).cloned().collect()
            }
        }
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }
}

pub struct LogBuffer {
    buffers: HashMap<String, CircularBuffer>,
    max_lines_per_service: usize,
}

impl LogBuffer {
    pub fn new(max_lines_per_service: usize) -> Self {
        Self {
            buffers: HashMap::new(),
            max_lines_per_service,
        }
    }

    pub fn append(&mut self, service: &str, line: LogLine) {
        let buffer = self.buffers
            .entry(service.to_string())
            .or_insert_with(|| CircularBuffer::new(self.max_lines_per_service));
        
        buffer.push(line);
    }

    pub fn get_lines(&self, service: &str, count: LogLineCount) -> Vec<LogLine> {
        self.buffers
            .get(service)
            .map(|buffer| buffer.get_lines(count))
            .unwrap_or_default()
    }

    pub fn get_all_lines(&self, service: &str) -> Vec<LogLine> {
        self.get_lines(service, LogLineCount::All)
    }

    pub fn clear_service(&mut self, service: &str) {
        if let Some(buffer) = self.buffers.get_mut(service) {
            buffer.clear();
        }
    }

    pub fn clear_all(&mut self) {
        self.buffers.clear();
    }

    pub fn services(&self) -> Vec<String> {
        self.buffers.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_log_line(content: &str, stream: LogStream) -> LogLine {
        LogLine {
            timestamp: SystemTime::now(),
            content: content.to_string(),
            stream,
        }
    }

    #[test]
    fn test_circular_buffer_new() {
        let buffer = CircularBuffer::new(10);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_circular_buffer_push() {
        let mut buffer = CircularBuffer::new(3);
        
        buffer.push(create_log_line("line 1", LogStream::Stdout));
        assert_eq!(buffer.len(), 1);
        
        buffer.push(create_log_line("line 2", LogStream::Stdout));
        assert_eq!(buffer.len(), 2);
        
        buffer.push(create_log_line("line 3", LogStream::Stdout));
        assert_eq!(buffer.len(), 3);
    }

    #[test]
    fn test_circular_buffer_eviction() {
        let mut buffer = CircularBuffer::new(3);
        
        buffer.push(create_log_line("line 1", LogStream::Stdout));
        buffer.push(create_log_line("line 2", LogStream::Stdout));
        buffer.push(create_log_line("line 3", LogStream::Stdout));
        
        // Buffer is full, next push should evict oldest
        buffer.push(create_log_line("line 4", LogStream::Stdout));
        
        assert_eq!(buffer.len(), 3);
        let lines = buffer.get_lines(LogLineCount::All);
        assert_eq!(lines[0].content, "line 2");
        assert_eq!(lines[1].content, "line 3");
        assert_eq!(lines[2].content, "line 4");
    }

    #[test]
    fn test_circular_buffer_get_all_lines() {
        let mut buffer = CircularBuffer::new(5);
        
        buffer.push(create_log_line("line 1", LogStream::Stdout));
        buffer.push(create_log_line("line 2", LogStream::Stderr));
        buffer.push(create_log_line("line 3", LogStream::Stdout));
        
        let lines = buffer.get_lines(LogLineCount::All);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].content, "line 1");
        assert_eq!(lines[1].content, "line 2");
        assert_eq!(lines[2].content, "line 3");
    }

    #[test]
    fn test_circular_buffer_get_last_n_lines() {
        let mut buffer = CircularBuffer::new(10);
        
        for i in 1..=5 {
            buffer.push(create_log_line(&format!("line {}", i), LogStream::Stdout));
        }
        
        let lines = buffer.get_lines(LogLineCount::Last(3));
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].content, "line 3");
        assert_eq!(lines[1].content, "line 4");
        assert_eq!(lines[2].content, "line 5");
    }

    #[test]
    fn test_circular_buffer_get_last_n_more_than_available() {
        let mut buffer = CircularBuffer::new(10);
        
        buffer.push(create_log_line("line 1", LogStream::Stdout));
        buffer.push(create_log_line("line 2", LogStream::Stdout));
        
        // Request more lines than available
        let lines = buffer.get_lines(LogLineCount::Last(10));
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].content, "line 1");
        assert_eq!(lines[1].content, "line 2");
    }

    #[test]
    fn test_circular_buffer_clear() {
        let mut buffer = CircularBuffer::new(5);
        
        buffer.push(create_log_line("line 1", LogStream::Stdout));
        buffer.push(create_log_line("line 2", LogStream::Stdout));
        
        assert_eq!(buffer.len(), 2);
        
        buffer.clear();
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_circular_buffer_stream_types() {
        let mut buffer = CircularBuffer::new(5);
        
        buffer.push(create_log_line("stdout line", LogStream::Stdout));
        buffer.push(create_log_line("stderr line", LogStream::Stderr));
        
        let lines = buffer.get_lines(LogLineCount::All);
        assert_eq!(lines.len(), 2);
        
        match lines[0].stream {
            LogStream::Stdout => {},
            _ => panic!("Expected Stdout"),
        }
        
        match lines[1].stream {
            LogStream::Stderr => {},
            _ => panic!("Expected Stderr"),
        }
    }

    // LogBuffer tests
    #[test]
    fn test_log_buffer_new() {
        let buffer = LogBuffer::new(100);
        assert_eq!(buffer.services().len(), 0);
    }

    #[test]
    fn test_log_buffer_append() {
        let mut buffer = LogBuffer::new(100);
        
        buffer.append("service1", create_log_line("line 1", LogStream::Stdout));
        buffer.append("service1", create_log_line("line 2", LogStream::Stdout));
        buffer.append("service2", create_log_line("line 1", LogStream::Stdout));
        
        let lines1 = buffer.get_all_lines("service1");
        let lines2 = buffer.get_all_lines("service2");
        
        assert_eq!(lines1.len(), 2);
        assert_eq!(lines2.len(), 1);
        assert_eq!(lines1[0].content, "line 1");
        assert_eq!(lines1[1].content, "line 2");
        assert_eq!(lines2[0].content, "line 1");
    }

    #[test]
    fn test_log_buffer_automatic_eviction() {
        let mut buffer = LogBuffer::new(3);
        
        buffer.append("service1", create_log_line("line 1", LogStream::Stdout));
        buffer.append("service1", create_log_line("line 2", LogStream::Stdout));
        buffer.append("service1", create_log_line("line 3", LogStream::Stdout));
        buffer.append("service1", create_log_line("line 4", LogStream::Stdout));
        
        let lines = buffer.get_all_lines("service1");
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].content, "line 2");
        assert_eq!(lines[1].content, "line 3");
        assert_eq!(lines[2].content, "line 4");
    }

    #[test]
    fn test_log_buffer_get_lines_with_count() {
        let mut buffer = LogBuffer::new(100);
        
        for i in 1..=10 {
            buffer.append("service1", create_log_line(&format!("line {}", i), LogStream::Stdout));
        }
        
        let lines = buffer.get_lines("service1", LogLineCount::Last(5));
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0].content, "line 6");
        assert_eq!(lines[4].content, "line 10");
    }

    #[test]
    fn test_log_buffer_get_lines_nonexistent_service() {
        let buffer = LogBuffer::new(100);
        
        let lines = buffer.get_all_lines("nonexistent");
        assert_eq!(lines.len(), 0);
    }

    #[test]
    fn test_log_buffer_multiple_services() {
        let mut buffer = LogBuffer::new(100);
        
        buffer.append("db", create_log_line("db log 1", LogStream::Stdout));
        buffer.append("api", create_log_line("api log 1", LogStream::Stdout));
        buffer.append("db", create_log_line("db log 2", LogStream::Stderr));
        buffer.append("api", create_log_line("api log 2", LogStream::Stdout));
        
        let db_lines = buffer.get_all_lines("db");
        let api_lines = buffer.get_all_lines("api");
        
        assert_eq!(db_lines.len(), 2);
        assert_eq!(api_lines.len(), 2);
        assert_eq!(db_lines[0].content, "db log 1");
        assert_eq!(api_lines[0].content, "api log 1");
    }

    #[test]
    fn test_log_buffer_clear_service() {
        let mut buffer = LogBuffer::new(100);
        
        buffer.append("service1", create_log_line("line 1", LogStream::Stdout));
        buffer.append("service2", create_log_line("line 1", LogStream::Stdout));
        
        buffer.clear_service("service1");
        
        let lines1 = buffer.get_all_lines("service1");
        let lines2 = buffer.get_all_lines("service2");
        
        assert_eq!(lines1.len(), 0);
        assert_eq!(lines2.len(), 1);
    }

    #[test]
    fn test_log_buffer_clear_all() {
        let mut buffer = LogBuffer::new(100);
        
        buffer.append("service1", create_log_line("line 1", LogStream::Stdout));
        buffer.append("service2", create_log_line("line 1", LogStream::Stdout));
        
        buffer.clear_all();
        
        assert_eq!(buffer.services().len(), 0);
        assert_eq!(buffer.get_all_lines("service1").len(), 0);
        assert_eq!(buffer.get_all_lines("service2").len(), 0);
    }

    #[test]
    fn test_log_buffer_services_list() {
        let mut buffer = LogBuffer::new(100);
        
        buffer.append("service1", create_log_line("line 1", LogStream::Stdout));
        buffer.append("service2", create_log_line("line 1", LogStream::Stdout));
        buffer.append("service3", create_log_line("line 1", LogStream::Stdout));
        
        let mut services = buffer.services();
        services.sort();
        
        assert_eq!(services.len(), 3);
        assert_eq!(services, vec!["service1", "service2", "service3"]);
    }

    #[test]
    fn test_log_buffer_per_service_limits() {
        let mut buffer = LogBuffer::new(5);
        
        // Add 10 lines to service1
        for i in 1..=10 {
            buffer.append("service1", create_log_line(&format!("s1 line {}", i), LogStream::Stdout));
        }
        
        // Add 3 lines to service2
        for i in 1..=3 {
            buffer.append("service2", create_log_line(&format!("s2 line {}", i), LogStream::Stdout));
        }
        
        let lines1 = buffer.get_all_lines("service1");
        let lines2 = buffer.get_all_lines("service2");
        
        // service1 should only have last 5 lines
        assert_eq!(lines1.len(), 5);
        assert_eq!(lines1[0].content, "s1 line 6");
        assert_eq!(lines1[4].content, "s1 line 10");
        
        // service2 should have all 3 lines
        assert_eq!(lines2.len(), 3);
        assert_eq!(lines2[0].content, "s2 line 1");
        assert_eq!(lines2[2].content, "s2 line 3");
    }

    // Concurrent access tests using std::sync
    #[test]
    fn test_log_buffer_concurrent_append() {
        use std::sync::{Arc, Mutex};
        use std::thread;
        
        let buffer = Arc::new(Mutex::new(LogBuffer::new(1000)));
        let mut handles = vec![];
        
        // Spawn 5 threads, each appending 20 lines
        for thread_id in 0..5 {
            let buffer_clone = Arc::clone(&buffer);
            let handle = thread::spawn(move || {
                for i in 0..20 {
                    let mut buf = buffer_clone.lock().unwrap();
                    buf.append(
                        "concurrent_service",
                        create_log_line(&format!("thread {} line {}", thread_id, i), LogStream::Stdout)
                    );
                }
            });
            handles.push(handle);
        }
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
        
        let buf = buffer.lock().unwrap();
        let lines = buf.get_all_lines("concurrent_service");
        
        // Should have 100 total lines (5 threads * 20 lines)
        assert_eq!(lines.len(), 100);
    }

    #[test]
    fn test_log_buffer_concurrent_read_write() {
        use std::sync::{Arc, Mutex};
        use std::thread;
        
        let buffer = Arc::new(Mutex::new(LogBuffer::new(1000)));
        
        // Writer thread
        let buffer_writer = Arc::clone(&buffer);
        let writer = thread::spawn(move || {
            for i in 0..50 {
                let mut buf = buffer_writer.lock().unwrap();
                buf.append("service", create_log_line(&format!("line {}", i), LogStream::Stdout));
            }
        });
        
        // Reader thread
        let buffer_reader = Arc::clone(&buffer);
        let reader = thread::spawn(move || {
            for _ in 0..10 {
                let buf = buffer_reader.lock().unwrap();
                let _lines = buf.get_all_lines("service");
                // Just reading, no assertions needed
            }
        });
        
        writer.join().unwrap();
        reader.join().unwrap();
        
        let buf = buffer.lock().unwrap();
        let lines = buf.get_all_lines("service");
        assert_eq!(lines.len(), 50);
    }
}
