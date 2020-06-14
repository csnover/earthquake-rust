This document offers some general guiding principles for the project. It should be updated over time to match community consensus about the direction of the project.

The main overarching goal is to create a usable implementation of the Director playback engine which can be used to play movies and run projectors authored in Director. This could be expanded to other engines in the future, if it makes sense (see non-goals).

# Goals

1. Provide an excellent user experience.
   * Prefer automatic detection over manual configuration.
   * Provide clear & up-to-date documentation, especially for configuration.
   * Prefer native UI over custom UI so every platform feels first class.
   * Ensure support for assistive technology wherever possible.
   * Ensure all user interfaces are properly internationalised.
   * Ensure errors are clear and include instructions on what to do next.
   * Recover gracefully from unexpected conditions instead of crashing or terminating, whenever possible.
   * Include opt-in enhancements and quality-of-life improvements, like advanced scalers, save states, and patches for known bad original assets.
   * Respond quickly to bug reports and feature requests from users.
   * Use automation to release continuously for all supported platforms.
   * Gate unstable features behind flags.
   * Monitor runtime resource usage and avoid adding technologies which require large amounts of processing power or memory.
2. Create a space where everyone can feel welcome, learn new things, and grow their skills.
   * Answer others’ questions, provide constructive feedback, and help eliminate barriers to personal growth.
   * Speak openly about your own progress and challenges.
   * Follow industry standard best practices and use modern tools (without cargo culting).
   * Work together to make decisions based on consensus. Ensure mutual understanding even among those who disagree.
   * Hold discussions and make decisions out in the open, where anyone can participate (except for code of conduct violations).
   * Follow the [code of conduct](./CODE_OF_CONDUCT.md).
3. Write code which is clean, accurate, clear, and safe.
   * Don’t commit any code based on guesswork. This is unfair to other contributors who will need to rewrite it later, and unfair to users who will have to deal with constantly broken stuff.
   * Don’t rely on implementation details of underlying hardware, even if the alternative is not a zero-cost abstraction.
   * Prefer easy-to-understand implementations over fast ones in non-critical code paths.
   * If code is non-idiomatic, document *why* in comments.
   * Add hooks for extended functionality instead of adding code directly that didn’t exist in the original functions.
4. Support other projects by building separate crates for common code if it makes sense to do so.

# Non-goals

1. Portability to esoteric (below 1% general usage) or obsolete (no longer supported by their manufacturer) operating systems or platforms.
2. Support `no_std` environments.
3. Reconstruct precisely every single feature of the original software if such features are inherently unsafe or can be implemented in a more user-friendly way.
4. Recreate library code when existing crates already exist and work well.
5. Restrict the number or kinds of supported genres/engines in the repository for ideological reasons.
6. Force all code to be written in a single language.
