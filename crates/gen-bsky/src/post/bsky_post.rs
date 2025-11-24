//! # BskyPost Submodule - Individual Post Management
//!
//! This submodule provides the core data structures for managing individual Bluesky posts
//! within the larger post management workflow. It handles post state tracking and provides
//! access to post data and metadata.
//!
//! ## Components
//!
//! - `BskyPostState`: Enum representing the lifecycle state of a post
//! - `BskyPost`: Container for post data, file information, and state tracking
//!
//! ## State Management
//!
//! Posts progress through a defined lifecycle:
//! 1. **Read**: Loaded from file and ready for processing
//! 2. **Posted**: Successfully published to Bluesky
//! 3. **Deleted**: File cleaned up after successful posting
//!
//! ## Integration
//!
//! This module is designed to work closely with the parent `Post` struct,
//! providing fine-grained control over individual post lifecycle management.

use std::path::PathBuf;

use bsky_sdk::api::app::bsky::feed::post::RecordData;

/// # BskyPostState - Post Lifecycle State Management
///
/// Represents the current state of a Bluesky post within the publishing workflow.
/// Posts transition through these states in a defined sequence during processing.
///
/// ## State Transitions
///
/// ```text
/// Read -> Posted -> Deleted
///  ^        |         |
///  |        v         v
///  +------ Error ----+
/// ```
///
/// ## States
///
/// - `Read`: Post has been loaded from file and is ready for publishing
/// - `Posted`: Post has been successfully published to Bluesky
/// - `Deleted`: Post file has been cleaned up after successful publishing
///
/// ## Design Characteristics
///
/// - Implements `PartialEq` and `Eq` for state comparison in filtering operations
/// - `Debug` and `Clone` for debugging and flexible usage patterns
/// - States are mutually exclusive - a post can only be in one state at a time
///
/// ## Usage in Filtering
///
/// The enum is commonly used with iterator filters to process posts by state:
///
/// ```rust,ignore
/// // Process only posts ready for publishing
/// posts.iter().filter(|p| p.state() == &BskyPostState::Read)
///
/// // Clean up successfully posted content
/// posts.iter().filter(|p| p.state() == &BskyPostState::Posted)
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BskyPostState {
    /// Post has been loaded from a `.post` file and is ready for publishing.
    ///
    /// This is the initial state when posts are loaded via `Post::load()`.
    /// Posts in this state are candidates for publishing to Bluesky.
    Read,

    /// Post has been successfully published to Bluesky.
    ///
    /// Posts transition to this state after successful API calls to Bluesky.
    /// This state indicates the post is live on the platform and the source
    /// file is eligible for cleanup.
    Posted,

    /// Post file has been deleted from the filesystem after successful publishing.
    ///
    /// This is the final state in the post lifecycle. Posts in this state
    /// have been successfully published and their source files removed to
    /// prevent duplicate processing.
    Deleted,
}

/// # BskyPost - Individual Post Container
///
/// Encapsulates a single Bluesky post with its associated metadata and state tracking.
/// This struct bridges the gap between file-based post storage and the Bluesky API.
///
/// ## Fields
///
/// - `post`: The actual Bluesky post data ready for API submission
/// - `file_path`: Path to the source `.post` file for cleanup operations
/// - `state`: Current lifecycle state of the post
///
/// ## Memory Characteristics
///
/// - Post content is stored in memory as a `RecordData` structure
/// - File path is an owned `PathBuf` for independent file operations
/// - State tracking adds minimal memory overhead
/// - Implements `Clone` for flexible collection management
///
/// ## Lifecycle Management
///
/// Posts are created in `Read` state and progress through the publishing workflow:
/// 1. Creation via `BskyPost::new()` with loaded post data
/// 2. State transitions managed by the parent `Post` struct
/// 3. File cleanup based on state tracking
///
/// ## Thread Safety
///
/// - Individual posts are not thread-safe due to mutable state
/// - File path operations require coordination for concurrent access
/// - Clone support enables distribution across threads if needed
///
/// ## Integration with Post Workflow
///
/// ```rust,ignore
/// // Created during file loading
/// let bsky_post = BskyPost::new(record_data, file_path);
///
/// // State managed during publishing
/// if publish_successful {
///     bsky_post.set_state(BskyPostState::Posted);
/// }
///
/// // Cleanup based on state
/// if bsky_post.state() == &BskyPostState::Posted {
///     fs::remove_file(bsky_post.file_path())?;
///     bsky_post.set_state(BskyPostState::Deleted);
/// }
/// ```
#[derive(Debug, Clone)]
pub(crate) struct BskyPost {
    /// The Bluesky post data ready for API submission
    post: RecordData,

    /// Path to the source `.post` file for cleanup operations
    file_path: PathBuf,

    /// Current lifecycle state of the post
    state: BskyPostState,
}

impl BskyPost {
    /// Creates a new BskyPost instance with the specified post data and file path.
    ///
    /// Initializes a post in the `Read` state, ready for processing through the
    /// publishing workflow. This constructor is typically called during the file
    /// loading phase when `.post` files are deserialized.
    ///
    /// ## Parameters
    ///
    /// - `post`: Bluesky `RecordData` containing the post content and metadata
    /// - `file_path`: Path to the source `.post` file for cleanup operations
    ///
    /// ## Returns
    ///
    /// A new `BskyPost` instance in `Read` state
    ///
    /// ## Initial State
    ///
    /// Posts are always created in `BskyPostState::Read` state, indicating they
    /// are loaded and ready for publishing but have not yet been processed.
    ///
    /// ## Memory Ownership
    ///
    /// - Takes ownership of both `post` data and `file_path`
    /// - No validation is performed on the post content or file path
    /// - File path is stored for later cleanup operations
    ///
    /// ## Examples
    ///
    /// ```rust,ignore
    /// use bsky_sdk::api::app::bsky::feed::post::RecordData;
    /// use std::path::PathBuf;
    ///
    /// // Typically called during file loading
    /// let post_data = serde_json::from_reader(file_reader)?;
    /// let file_path = PathBuf::from("./posts/example.post");
    /// let bsky_post = BskyPost::new(post_data, file_path);
    ///
    /// assert_eq!(bsky_post.state(), &BskyPostState::Read);
    /// ```
    pub(crate) fn new(post: RecordData, file_path: PathBuf) -> Self {
        BskyPost {
            post,
            file_path,
            state: BskyPostState::Read,
        }
    }

    /// Returns a reference to the Bluesky post data.
    ///
    /// Provides access to the underlying `RecordData` structure containing
    /// the post content, metadata, and other Bluesky-specific fields ready
    /// for API submission.
    ///
    /// ## Returns
    ///
    /// An immutable reference to the `RecordData` structure
    ///
    /// ## Usage
    ///
    /// The returned data can be:
    /// - Submitted directly to Bluesky APIs
    /// - Inspected for logging and debugging
    /// - Cloned for API calls that require owned data
    ///
    /// ## Post Data Structure
    ///
    /// The `RecordData` typically contains:
    /// - `text`: The post content
    /// - `createdAt`: Timestamp of post creation
    /// - `langs`: Language codes for the post
    /// - Additional metadata fields
    ///
    /// ## Performance
    ///
    /// - **Time Complexity**: O(1) constant time access
    /// - **Memory**: Returns reference, no additional allocation
    /// - **Cost**: Minimal overhead for reference access
    ///
    /// ## Examples
    ///
    /// ```rust,ignore
    /// // Access post content for logging
    /// println!("Post content: {}", bsky_post.post().text);
    ///
    /// // Submit to Bluesky API
    /// let result = agent.create_record(bsky_post.post().clone()).await;
    ///
    /// // Check post metadata
    /// if let Some(langs) = &bsky_post.post().langs {
    ///     println!("Post languages: {:?}", langs);
    /// }
    /// ```
    pub(crate) fn post(&self) -> &RecordData {
        &self.post
    }

    /// Returns a reference to the source file path.
    ///
    /// Provides access to the path of the original `.post` file that contained
    /// this post's data. This path is used for cleanup operations after
    /// successful publishing.
    ///
    /// ## Returns
    ///
    /// An immutable reference to the `PathBuf` of the source file
    ///
    /// ## Usage
    ///
    /// The file path is used for:
    /// - File deletion after successful posting
    /// - Logging and error reporting
    /// - Debugging and troubleshooting
    /// - Progress tracking and status reporting
    ///
    /// ## File Operations
    ///
    /// The path can be used with standard file operations:
    /// - `fs::remove_file()` for cleanup
    /// - `fs::metadata()` for file information
    /// - Path manipulation for backup operations
    ///
    /// ## Performance
    ///
    /// - **Time Complexity**: O(1) constant time access
    /// - **Memory**: Returns reference, no additional allocation
    /// - **File I/O**: No file system access, just path reference
    ///
    /// ## Examples
    ///
    /// ```rust,ignore
    /// // File cleanup after successful posting
    /// if bsky_post.state() == &BskyPostState::Posted {
    ///     fs::remove_file(bsky_post.file_path())?;
    /// }
    ///
    /// // Logging for progress tracking
    /// println!("Processing: {}", bsky_post.file_path().display());
    ///
    /// // Error reporting with file context
    /// eprintln!("Failed to process: {}", bsky_post.file_path().to_string_lossy());
    /// ```
    pub(crate) fn file_path(&self) -> &PathBuf {
        &self.file_path
    }

    /// Returns the current state of the post.
    ///
    /// Provides access to the post's current position in the publishing lifecycle.
    /// This state information is used for filtering, progress tracking, and
    /// workflow control.
    ///
    /// ## Returns
    ///
    /// An immutable reference to the current `BskyPostState`
    ///
    /// ## State Information
    ///
    /// Possible states:
    /// - `Read`: Post loaded and ready for publishing
    /// - `Posted`: Post successfully published to Bluesky
    /// - `Deleted`: Post file cleaned up after successful publishing
    ///
    /// ## Usage Patterns
    ///
    /// States are commonly used for:
    /// - Filtering posts for specific operations
    /// - Progress tracking and reporting
    /// - Conditional processing logic
    /// - Workflow state validation
    ///
    /// ## Performance
    ///
    /// - **Time Complexity**: O(1) constant time access
    /// - **Memory**: Returns reference, no additional allocation
    /// - **Comparison**: States implement `PartialEq` for efficient comparison
    ///
    /// ## Examples
    ///
    /// ```rust,ignore
    /// // Filter posts by state
    /// let ready_posts: Vec<_> = posts.iter()
    ///     .filter(|p| p.state() == &BskyPostState::Read)
    ///     .collect();
    ///
    /// // Conditional processing
    /// match bsky_post.state() {
    ///     BskyPostState::Read => publish_post(&bsky_post),
    ///     BskyPostState::Posted => cleanup_file(&bsky_post),
    ///     BskyPostState::Deleted => println!("Already processed"),
    /// }
    ///
    /// // Progress reporting
    /// let posted_count = posts.iter()
    ///     .filter(|p| p.state() == &BskyPostState::Posted)
    ///     .count();
    /// ```
    pub(crate) fn state(&self) -> &BskyPostState {
        &self.state
    }

    /// Updates the post state to the specified new state.
    ///
    /// Transitions the post to a new state in the publishing lifecycle.
    /// This method is typically called by the parent `Post` struct as
    /// posts progress through the workflow.
    ///
    /// ## Parameters
    ///
    /// - `new_state`: The target state for the post
    ///
    /// ## State Transitions
    ///
    /// While any transition is technically allowed, the normal workflow follows:
    /// ```text
    /// Read -> Posted -> Deleted
    /// ```
    ///
    /// ## Mutation
    ///
    /// This is the only method that mutates the post state, ensuring controlled
    /// state management through the publishing workflow.
    ///
    /// ## Usage Context
    ///
    /// Typically called in these scenarios:
    /// - After successful API submission: `set_state(BskyPostState::Posted)`
    /// - After file cleanup: `set_state(BskyPostState::Deleted)`
    /// - Error recovery scenarios (less common)
    ///
    /// ## Performance
    ///
    /// - **Time Complexity**: O(1) constant time operation
    /// - **Memory**: Simple field assignment, no allocation
    /// - **Side Effects**: Only modifies internal state
    ///
    /// ## Thread Safety
    ///
    /// - Requires mutable access, preventing concurrent state changes
    /// - State changes are atomic at the individual post level
    /// - No synchronization provided for multi-threaded access
    ///
    /// ## Examples
    ///
    /// ```rust,ignore
    /// // Typical usage in publishing workflow
    /// let result = agent.create_record(bsky_post.post().clone()).await;
    /// if result.is_ok() {
    ///     bsky_post.set_state(BskyPostState::Posted);
    /// }
    ///
    /// // File cleanup workflow
    /// if bsky_post.state() == &BskyPostState::Posted {
    ///     fs::remove_file(bsky_post.file_path())?;
    ///     bsky_post.set_state(BskyPostState::Deleted);
    /// }
    ///
    /// // Error recovery (if needed)
    /// if post_failed_to_publish {
    ///     bsky_post.set_state(BskyPostState::Read); // Reset for retry
    /// }
    /// ```
    pub(crate) fn set_state(&mut self, new_state: BskyPostState) {
        self.state = new_state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bsky_sdk::api::app::bsky::feed::post::RecordData;
    use std::path::PathBuf;

    /// Helper function to create test RecordData
    fn create_test_record_data(text: &str) -> RecordData {
        use bsky_sdk::api::types::string::Datetime;
        use bsky_sdk::api::types::string::Language;

        RecordData {
            text: text.to_string(),
            created_at: Datetime::now(),
            langs: Some(vec![Language::new("en".to_string()).unwrap()]),
            reply: None,
            embed: None,
            facets: None,
            tags: None,
            entities: None,
            labels: None,
        }
    }

    /// Helper function to create a test BskyPost
    fn create_test_bsky_post() -> BskyPost {
        let record = create_test_record_data("Test post content");
        let path = PathBuf::from("/test/path/post.post");
        BskyPost::new(record, path)
    }

    // BskyPostState tests
    #[test]
    fn test_bsky_post_state_equality() {
        // Test PartialEq implementation
        assert_eq!(BskyPostState::Read, BskyPostState::Read);
        assert_eq!(BskyPostState::Posted, BskyPostState::Posted);
        assert_eq!(BskyPostState::Deleted, BskyPostState::Deleted);

        assert_ne!(BskyPostState::Read, BskyPostState::Posted);
        assert_ne!(BskyPostState::Posted, BskyPostState::Deleted);
        assert_ne!(BskyPostState::Read, BskyPostState::Deleted);
    }

    #[test]
    fn test_bsky_post_state_debug() {
        // Test Debug implementation
        let read_debug = format!("{:?}", BskyPostState::Read);
        let posted_debug = format!("{:?}", BskyPostState::Posted);
        let deleted_debug = format!("{:?}", BskyPostState::Deleted);

        assert_eq!(read_debug, "Read");
        assert_eq!(posted_debug, "Posted");
        assert_eq!(deleted_debug, "Deleted");
    }

    #[test]
    fn test_bsky_post_state_clone() {
        // Test Clone implementation
        let original = BskyPostState::Posted;
        let cloned = original.clone();
        assert_eq!(original, cloned);

        // Verify they are independent (enum copies are independent by nature)
        let _modified_clone = BskyPostState::Deleted;
        assert_eq!(original, BskyPostState::Posted);
    }

    #[test]
    fn test_bsky_post_state_filtering() {
        // Test usage in filtering scenarios
        let states = [
            BskyPostState::Read,
            BskyPostState::Posted,
            BskyPostState::Read,
            BskyPostState::Deleted,
            BskyPostState::Posted,
        ];

        let read_count = states.iter().filter(|&s| s == &BskyPostState::Read).count();
        let posted_count = states
            .iter()
            .filter(|&s| s == &BskyPostState::Posted)
            .count();
        let deleted_count = states
            .iter()
            .filter(|&s| s == &BskyPostState::Deleted)
            .count();

        assert_eq!(read_count, 2);
        assert_eq!(posted_count, 2);
        assert_eq!(deleted_count, 1);
    }

    // BskyPost constructor tests
    #[test]
    fn test_bsky_post_new() {
        let record = create_test_record_data("Test content");
        let path = PathBuf::from("/test/example.post");

        let bsky_post = BskyPost::new(record.clone(), path.clone());

        assert_eq!(bsky_post.post().text, "Test content");
        assert_eq!(bsky_post.file_path(), &path);
        assert_eq!(bsky_post.state(), &BskyPostState::Read);
    }

    #[test]
    fn test_bsky_post_new_initial_state() {
        // Verify posts always start in Read state
        let bsky_post = create_test_bsky_post();
        assert_eq!(bsky_post.state(), &BskyPostState::Read);
    }

    #[test]
    fn test_bsky_post_new_with_different_content() {
        let texts = vec![
            "Short",
            "A much longer post with more content",
            "",
            "🦀 Rust with emoji",
        ];

        for text in texts {
            let record = create_test_record_data(text);
            let path = PathBuf::from(format!("/test/{}.post", text.len()));
            let bsky_post = BskyPost::new(record, path);

            assert_eq!(bsky_post.post().text, text);
            assert_eq!(bsky_post.state(), &BskyPostState::Read);
        }
    }

    // Accessor method tests
    #[test]
    fn test_bsky_post_post_accessor() {
        let original_text = "Test post for accessor";
        let record = create_test_record_data(original_text);
        let bsky_post = BskyPost::new(record, PathBuf::from("/test.post"));

        let retrieved_post = bsky_post.post();
        assert_eq!(retrieved_post.text, original_text);

        // Verify it returns a reference
        let ptr1 = retrieved_post as *const RecordData;
        let ptr2 = bsky_post.post() as *const RecordData;
        assert_eq!(ptr1, ptr2);
    }

    #[test]
    fn test_bsky_post_file_path_accessor() {
        let original_path = PathBuf::from("/some/test/path.post");
        let bsky_post = BskyPost::new(create_test_record_data("test"), original_path.clone());

        let retrieved_path = bsky_post.file_path();
        assert_eq!(retrieved_path, &original_path);

        // Verify it returns a reference
        let ptr1 = retrieved_path as *const PathBuf;
        let ptr2 = bsky_post.file_path() as *const PathBuf;
        assert_eq!(ptr1, ptr2);
    }

    #[test]
    fn test_bsky_post_state_accessor() {
        let mut bsky_post = create_test_bsky_post();

        // Test initial state
        assert_eq!(bsky_post.state(), &BskyPostState::Read);

        // Test state after modification
        bsky_post.set_state(BskyPostState::Posted);
        assert_eq!(bsky_post.state(), &BskyPostState::Posted);

        bsky_post.set_state(BskyPostState::Deleted);
        assert_eq!(bsky_post.state(), &BskyPostState::Deleted);
    }

    // State transition tests
    #[test]
    fn test_bsky_post_set_state() {
        let mut bsky_post = create_test_bsky_post();

        // Test normal workflow transitions
        assert_eq!(bsky_post.state(), &BskyPostState::Read);

        bsky_post.set_state(BskyPostState::Posted);
        assert_eq!(bsky_post.state(), &BskyPostState::Posted);

        bsky_post.set_state(BskyPostState::Deleted);
        assert_eq!(bsky_post.state(), &BskyPostState::Deleted);
    }

    #[test]
    fn test_bsky_post_set_state_non_linear_transitions() {
        let mut bsky_post = create_test_bsky_post();

        // Test non-linear transitions (should be allowed)
        bsky_post.set_state(BskyPostState::Deleted);
        assert_eq!(bsky_post.state(), &BskyPostState::Deleted);

        bsky_post.set_state(BskyPostState::Read);
        assert_eq!(bsky_post.state(), &BskyPostState::Read);

        bsky_post.set_state(BskyPostState::Posted);
        assert_eq!(bsky_post.state(), &BskyPostState::Posted);
    }

    #[test]
    fn test_bsky_post_set_state_same_state() {
        let mut bsky_post = create_test_bsky_post();

        // Setting the same state should work
        assert_eq!(bsky_post.state(), &BskyPostState::Read);
        bsky_post.set_state(BskyPostState::Read);
        assert_eq!(bsky_post.state(), &BskyPostState::Read);

        bsky_post.set_state(BskyPostState::Posted);
        bsky_post.set_state(BskyPostState::Posted);
        assert_eq!(bsky_post.state(), &BskyPostState::Posted);
    }

    // Clone and Debug tests
    #[test]
    fn test_bsky_post_clone() {
        let original = create_test_bsky_post();
        let cloned = original.clone();

        // Verify content is the same
        assert_eq!(original.post().text, cloned.post().text);
        assert_eq!(original.file_path(), cloned.file_path());
        assert_eq!(original.state(), cloned.state());

        // Verify they are independent
        let mut cloned_modified = cloned.clone();
        cloned_modified.set_state(BskyPostState::Posted);

        assert_eq!(original.state(), &BskyPostState::Read);
        assert_eq!(cloned_modified.state(), &BskyPostState::Posted);
    }

    #[test]
    fn test_bsky_post_debug() {
        let bsky_post = create_test_bsky_post();
        let debug_str = format!("{bsky_post:?}");

        // Should contain struct name and basic info
        assert!(debug_str.contains("BskyPost"));
        // Debug format may vary, so just check it's not empty
        assert!(!debug_str.is_empty());
    }

    // Edge cases and boundary tests
    #[test]
    fn test_bsky_post_with_empty_text() {
        let record = create_test_record_data("");
        let bsky_post = BskyPost::new(record, PathBuf::from("/empty.post"));

        assert_eq!(bsky_post.post().text, "");
        assert_eq!(bsky_post.state(), &BskyPostState::Read);
    }

    #[test]
    fn test_bsky_post_with_unicode_content() {
        let unicode_text = "Hello 世界 🌍 Здравствуй мир";
        let record = create_test_record_data(unicode_text);
        let bsky_post = BskyPost::new(record, PathBuf::from("/unicode.post"));

        assert_eq!(bsky_post.post().text, unicode_text);
    }

    #[test]
    fn test_bsky_post_with_long_path() {
        let long_path = PathBuf::from("/very/deep/directory/structure/with/many/levels/post.post");
        let bsky_post = BskyPost::new(create_test_record_data("test"), long_path.clone());

        assert_eq!(bsky_post.file_path(), &long_path);
    }

    #[test]
    fn test_bsky_post_with_relative_path() {
        let relative_path = PathBuf::from("./posts/relative.post");
        let bsky_post = BskyPost::new(create_test_record_data("test"), relative_path.clone());

        assert_eq!(bsky_post.file_path(), &relative_path);
    }

    // State filtering integration tests
    #[test]
    fn test_multiple_posts_state_filtering() {
        let mut posts = [
            create_test_bsky_post(),
            create_test_bsky_post(),
            create_test_bsky_post(),
            create_test_bsky_post(),
        ];

        // Set different states
        posts[1].set_state(BskyPostState::Posted);
        posts[2].set_state(BskyPostState::Deleted);
        posts[3].set_state(BskyPostState::Posted);

        // Test filtering by state
        let read_posts: Vec<_> = posts
            .iter()
            .filter(|p| p.state() == &BskyPostState::Read)
            .collect();
        let posted_posts: Vec<_> = posts
            .iter()
            .filter(|p| p.state() == &BskyPostState::Posted)
            .collect();
        let deleted_posts: Vec<_> = posts
            .iter()
            .filter(|p| p.state() == &BskyPostState::Deleted)
            .collect();

        assert_eq!(read_posts.len(), 1);
        assert_eq!(posted_posts.len(), 2);
        assert_eq!(deleted_posts.len(), 1);
    }

    #[test]
    fn test_state_transition_workflow() {
        let mut bsky_post = create_test_bsky_post();

        // Simulate complete workflow

        // 1. Initial state after loading
        assert_eq!(bsky_post.state(), &BskyPostState::Read);

        // 2. After successful posting
        bsky_post.set_state(BskyPostState::Posted);
        assert_eq!(bsky_post.state(), &BskyPostState::Posted);

        // 3. After file cleanup
        bsky_post.set_state(BskyPostState::Deleted);
        assert_eq!(bsky_post.state(), &BskyPostState::Deleted);
    }

    // Memory and performance tests
    #[test]
    fn test_many_posts_creation() {
        let posts: Vec<BskyPost> = (0..1000)
            .map(|i| {
                let record = create_test_record_data(&format!("Post {i}"));
                let path = PathBuf::from(format!("/test/post{i}.post"));
                BskyPost::new(record, path)
            })
            .collect();

        assert_eq!(posts.len(), 1000);

        // Verify all start in Read state
        let read_count = posts
            .iter()
            .filter(|p| p.state() == &BskyPostState::Read)
            .count();
        assert_eq!(read_count, 1000);

        // Verify content is correct
        assert_eq!(posts[0].post().text, "Post 0");
        assert_eq!(posts[999].post().text, "Post 999");
    }

    #[test]
    fn test_state_changes_performance() {
        let mut posts: Vec<BskyPost> = (0..100)
            .map(|i| {
                let record = create_test_record_data(&format!("Post {i}"));
                let path = PathBuf::from(format!("/test/post{i}.post"));
                BskyPost::new(record, path)
            })
            .collect();

        // Simulate state changes
        for post in posts.iter_mut().take(30) {
            post.set_state(BskyPostState::Posted);
        }

        for post in posts.iter_mut().skip(30).take(20) {
            post.set_state(BskyPostState::Deleted);
        }

        // Count states
        let read_count = posts
            .iter()
            .filter(|p| p.state() == &BskyPostState::Read)
            .count();
        let posted_count = posts
            .iter()
            .filter(|p| p.state() == &BskyPostState::Posted)
            .count();
        let deleted_count = posts
            .iter()
            .filter(|p| p.state() == &BskyPostState::Deleted)
            .count();

        assert_eq!(read_count, 50);
        assert_eq!(posted_count, 30);
        assert_eq!(deleted_count, 20);
    }
}
