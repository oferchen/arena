# Analytics

This crate provides a simple in-memory store for analytics events used by
Arena. The store keeps a limited number of events to avoid unbounded memory
usage.

By default, up to 10,000 events are retained. You can override this limit by
setting the `ARENA_ANALYTICS_MAX_EVENTS` environment variable to the desired
capacity.
