# 0018: Library data isolation and newer-schema errors

Date: 2026-11-21. Status: stacked above the track radio work.

The published 0.1.0 release could not open its music library on a machine used
for development. Debug builds had migrated the shared application database to
schema version 4, while the release understood at most version 3. The version
guard returned the same storage error as a permission or disk failure, so the
window only said "Could not load the music library" and the folder picker
reported an error without a reason. A clean machine was never affected: it has
no database, so the release creates one at its own version.

Two changes make this class of failure both preventable and diagnosable.

Debug builds now keep their data in a `dev` subdirectory of the application data
directory. The library database, the copied background image and the mpv session
log all follow the same rule, so a development schema upgrade can never change
the database that an installed release opens. Release builds keep the original
location, and existing users see no migration. The trade-off is that a
development machine has two libraries: the release one and the debug one.
Moving an existing development database into `dev` restores the debug data;
nothing is moved automatically.

A database written by a newer application version now returns the dedicated
`library_too_new` code instead of `storage_failed`. The setup phase keeps the
open error in the managed library state, so settings, library entries and the
folder picker all repeat the same actionable message: update the application or
restore an older library backup. The error is not retryable, because retrying
or choosing another folder cannot change the database version. The renderer
shows the specific message only for this code; other library failures keep
their previous wording.

A clean install, an upgrade from an older schema and a downgrade with a
restored backup all remain supported. A downgrade without a backup is still
impossible, but it is now explained instead of appearing as a generic storage
failure.
