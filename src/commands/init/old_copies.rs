//! The skills, grid agents and skill library that rex 0.4 to 0.7 copied into
//! `.claude/` and `.rex/skills/`: a copy the person never changed is removed, because it hides or doubles what the plugin ships.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use super::print_line_to_stderr;
use crate::bundle;
use crate::commands::links::is_symlink;
use crate::commands::stdout::print_line_to_stdout;
use crate::error::RexError;

/// One folder an old `rex init` copied into, and what each copy in it is.
struct OldRoot {
    /// The folder, from the repo root.
    folder: &'static str,
    /// The root of the bundle whose names the plugin ships in its place.
    shipped: &'static str,
    /// True when a copy is a folder (a skill), false when it is one file (an agent).
    is_folder: bool,
    /// What a copy is, as the warnings name it.
    noun: &'static str,
    /// What a kept copy does to the one the plugin ships, after "so".
    effect: &'static str,
}

/// The library first: the plugin reads `.rex/skills/` before its own library.
/// Then the skills and agents, which Claude Code lists beside the plugin's own.
const OLD_ROOTS: [OldRoot; 3] = [
    OldRoot {
        folder: ".rex/skills",
        shipped: bundle::LIBRARY_ROOT,
        is_folder: true,
        noun: "skill",
        effect: "it hides",
    },
    OldRoot {
        folder: ".claude/skills",
        shipped: bundle::SKILLS_ROOT,
        is_folder: true,
        noun: "skill",
        effect: "the session lists it beside",
    },
    OldRoot {
        folder: ".claude/agents",
        shipped: bundle::AGENTS_ROOT,
        is_folder: false,
        noun: "agent",
        effect: "the session lists it beside",
    },
];

/// The one file in a copy that is not a change of the person's: macOS Finder
/// writes it into any folder it shows.
const FINDER_FILE: &str = ".DS_Store";

/// Remove each copy in the folders of [`OLD_ROOTS`] in the repo at `cwd` that
/// has the name of something the plugin ships and holds only files that an
/// old `rex init` wrote there byte for byte. Each removal is one line on
/// stdout. A copy with any other file, a changed one, or a link in it is kept,
/// with a warning that it hides the shipped one. A copy with a name the plugin
/// does not ship, such as a skill a job wrote or a feedback fork, is the
/// person's and is left alone in silence. A linked root folder may be shared
/// with other repos, so it is never looked into.
///
/// The sandbox never lets a command write `.claude/skills` or `.claude/agents`;
/// `rex init` runs outside it (`rex init*` is an excluded command), so it can.
/// A copy that cannot be read or removed is a warning, never a failure.
///
/// # Errors
/// [`RexError::Stdout`] if a line cannot be printed.
pub(super) fn remove_old_copies(cwd: &Path) -> Result<(), RexError> {
    for root in &OLD_ROOTS {
        remove_in(cwd, root)?;
    }
    Ok(())
}

fn remove_in(cwd: &Path, root: &OldRoot) -> Result<(), RexError> {
    let folder = cwd.join(root.folder);
    let Ok(entries) = fs::read_dir(&folder) else {
        return Ok(());
    };
    let shipped = shipped_names(root.shipped);
    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| shipped.contains(name.as_str()))
        .collect();
    names.sort();
    if names.is_empty() {
        return Ok(());
    }
    if is_symlink(&folder) {
        print_line_to_stderr(&format!(
            "rex init: warning: {} is a symbolic link, so rex did not remove the copies an older rex installed there; for a copy there, {} the {} of the same name that the plugin ships",
            root.folder, root.effect, root.noun
        ));
        return Ok(());
    }
    for name in names {
        let copy = folder.join(&name);
        let place = format!("{}/{name}", root.folder);
        let Ok(meta) = fs::symlink_metadata(&copy) else {
            continue;
        };
        let (shown, is_unchanged) = if root.is_folder {
            if !meta.is_dir() {
                continue;
            }
            (format!("{place}/"), is_unchanged_folder(&copy, &place))
        } else {
            if !meta.is_file() {
                continue;
            }
            (place.clone(), is_old_file(&place, &copy))
        };
        if !is_unchanged {
            let what = if root.is_folder { "folder" } else { "file" };
            print_line_to_stderr(&format!(
                "rex init: warning: kept {shown}: it differs from the copy an older rex installed, so {} the {name} {} that the plugin ships; to use the plugin's, move your changes out and delete that {what}",
                root.effect, root.noun
            ));
            continue;
        }
        let removed = if root.is_folder {
            fs::remove_dir_all(&copy)
        } else {
            fs::remove_file(&copy)
        };
        match removed {
            Ok(()) => print_line_to_stdout(&format!(
                "rex init: removed {shown}: it was an unchanged copy from an older rex, and the plugin ships {name} now"
            ))?,
            Err(source) => print_line_to_stderr(&format!(
                "rex init: warning: could not remove {shown}, an unchanged copy from an older rex of the {name} {} the plugin ships: {source}; delete it",
                root.noun
            )),
        }
    }
    Ok(())
}

/// The names right below `shipped`, one of the bundle's roots: a skill's
/// folder or an agent's file.
fn shipped_names(shipped: &str) -> BTreeSet<String> {
    let root = Path::new(shipped);
    bundle::walk()
        .into_iter()
        .filter_map(|(path, _)| {
            let inside = path.strip_prefix(root).ok()?;
            let name = inside.components().next()?.as_os_str().to_str()?;
            Some(name.to_owned())
        })
        .collect()
}

/// True when every file below `copy`, found at `place` from the repo root, is
/// a file an old `rex init` wrote at that place with those bytes, and nothing
/// in it is a link or unreadable. The Finder's file is not counted.
fn is_unchanged_folder(copy: &Path, place: &str) -> bool {
    let mut pending = vec![(copy.to_path_buf(), place.to_owned())];
    while let Some((dir, prefix)) = pending.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            return false;
        };
        for entry in entries {
            let Ok(entry) = entry else {
                return false;
            };
            let Some(file_name) = entry.file_name().into_string().ok() else {
                return false;
            };
            let Ok(kind) = entry.file_type() else {
                return false;
            };
            let path = format!("{prefix}/{file_name}");
            if kind.is_dir() {
                pending.push((entry.path(), path));
            } else {
                let is_old = kind.is_file()
                    && (file_name == FINDER_FILE || is_old_file(&path, &entry.path()));
                if !is_old {
                    return false;
                }
            }
        }
    }
    true
}

/// True when the file at `on_disk` has the bytes an old release wrote at
/// `place`, its path from the repo root.
fn is_old_file(place: &str, on_disk: &Path) -> bool {
    let Ok(bytes) = fs::read(on_disk) else {
        return false;
    };
    let hash: String = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    OLD_FILES.binary_search(&(place, hash.as_str())).is_ok()
}

/// The SHA-256 of each file that rex 0.4.0 to 0.7.0 copied into a folder of
/// [`OLD_ROOTS`], as `(path from the repo root, lower-case hex)`, sorted.
///
/// Made once from the release commits with `git show <commit>:<path>`, for
/// 0.4.0 (a8164b4), 0.5.0 (4485a19), 0.6.0 (0ffb959) and 0.7.0 (7deda22):
/// their bundles embedded these folders, and each `rex init` wrote the bytes
/// unchanged. Those releases never change, so neither does this table. A
/// path with two hashes changed between releases. 0.7 never replaced a file
/// that differed without `--force`, so a repo can hold any of them.
const OLD_FILES: [(&str, &str); 195] = [
    (
        ".claude/agents/model-high-effort-high.md",
        "959453d090ae346bb690e0fc281b28f40b8e546ff342e456a4d1a81887ba2c02",
    ),
    (
        ".claude/agents/model-high-effort-low.md",
        "5552ea77a5cd7616b85b935af2fb1ec0847f73d9fad1518818108725beeeff5f",
    ),
    (
        ".claude/agents/model-high-effort-max.md",
        "9ae1325d0bbb50f6eabd99fb1dd1463acfa69a498ba8937eaf1a5f5e023a586a",
    ),
    (
        ".claude/agents/model-high-effort-medium.md",
        "08218398a1e9f8f67ca5156bc6b244ae7b93ea11dee6c3008d9ff991001acd7b",
    ),
    (
        ".claude/agents/model-high-effort-xhigh.md",
        "00ab3bf45169e57db1ede863ed666b8e08060ab48a56b10d1c812438c6a4777e",
    ),
    (
        ".claude/agents/model-low-effort-high.md",
        "914f60ade2720964e1e96502caf8b7bcc7a16726bd30c6d82e3fb9e3cd174022",
    ),
    (
        ".claude/agents/model-low-effort-low.md",
        "f7aa8fc546deb7767522dc7181f2fcd708346ce422d2b2b40ae6905133fd663f",
    ),
    (
        ".claude/agents/model-low-effort-max.md",
        "d9d7b498c326a92bb8bf1700cb518de91192c374969338a286206c5ee55533d0",
    ),
    (
        ".claude/agents/model-low-effort-medium.md",
        "8642e48bfc43b967cd29d4233dc0518b347a8f50c0d831e9164299ce21a39cdc",
    ),
    (
        ".claude/agents/model-low-effort-xhigh.md",
        "db825b302a1fef1f497e2b88784d3fd71e82e808a410b76ca7bc0752d693dda9",
    ),
    (
        ".claude/agents/model-medium-effort-high.md",
        "adb68366e600a12819cbabca9201eaa2360051fed6cfd620e3ae7cb1ea867151",
    ),
    (
        ".claude/agents/model-medium-effort-low.md",
        "c6bc5b80e3180a9a863309ffa473a90de01ecd01e5fc72d8b9fbc529590508f0",
    ),
    (
        ".claude/agents/model-medium-effort-max.md",
        "6cc598899422e910fa76d94cb919497ff416d2484037105aef4491b1ec74b669",
    ),
    (
        ".claude/agents/model-medium-effort-medium.md",
        "8ffe16d52208a06081a75a0cb9b6196c8b125f375278537d7ecdb4129840f099",
    ),
    (
        ".claude/agents/model-medium-effort-xhigh.md",
        "2515021f82e5117dbaa69d9e05e25ca2cc6ddc0637f538d628ba0a70e4213f71",
    ),
    (
        ".claude/agents/model-xhigh-effort-high.md",
        "6600519b3b1b0efb2e61758618ac681c5b42eab65468ae32d0b328ffd81d421d",
    ),
    (
        ".claude/agents/model-xhigh-effort-low.md",
        "88defd07dd8e01664ae68365ac0eacb89680436ee633cb3d088103c7deca5041",
    ),
    (
        ".claude/agents/model-xhigh-effort-max.md",
        "02e8b26fd1023bc9f30137534520e09b579fdcc6f0e2dc1cb055980108ee44e4",
    ),
    (
        ".claude/agents/model-xhigh-effort-medium.md",
        "575c7a0022bf7ffefbe218448420cb2909e702ce2f4bf8d821db2dd627f8c016",
    ),
    (
        ".claude/agents/model-xhigh-effort-xhigh.md",
        "35c56b44199b560ffdff98f5692b9ac83ba7bd667076559a8e16d9a6c0e5842b",
    ),
    (
        ".claude/skills/orchestration-builder/SKILL.md",
        "01845e3684f1c262182762d7db0b52e068898c285e27a778d2af1c3c992e8257",
    ),
    (
        ".claude/skills/orchestration-builder/SKILL.md",
        "885d364afcc0295a348d514dffbecd7e4841c2606c43e971a8912cdea84c1e04",
    ),
    (
        ".claude/skills/orchestration-builder/SKILL.md",
        "b74d73b08e898d32adf47c49a6c99f94c4691edbc38bc8658b9c291936454ba8",
    ),
    (
        ".claude/skills/orchestration-builder/classify.md",
        "60e9eb4b9d216a7d90903acdb50a4899364c681b0fb39c2e5924d4870bfae86e",
    ),
    (
        ".claude/skills/orchestration-builder/classify.md",
        "c46acb804a561ba3d492e59452b56d3775abaa2cf69f546938bff94361269e7f",
    ),
    (
        ".claude/skills/orchestration-builder/decide.md",
        "0080c5af7f456650c4bc5e87cda99b1e357a9a2fcb55cfa7d2adfb5e43f5a4a7",
    ),
    (
        ".claude/skills/orchestration-builder/decide.md",
        "898712f786ba2a6cd93120c40265bf37bee7962ad4e17e63e8b5e37745161502",
    ),
    (
        ".claude/skills/orchestration-builder/decide.md",
        "cfe69809d149713a7ee4501ea2e3dbccfd1f8503c35d88ebc9f8e5b0950932a4",
    ),
    (
        ".claude/skills/orchestration-builder/open-job.md",
        "296785a172d7b385d6b75ebc249fdba4e9e00dcda9a97c1d6e860b083353c502",
    ),
    (
        ".claude/skills/orchestration-builder/plan-jobs.md",
        "529a2eb56c4f385ef7537ed062d19e1b2fd2c3b38eb33c5851b2f18d55f7d494",
    ),
    (
        ".claude/skills/orchestration-builder/plan-jobs.md",
        "8b825f2c1d9bbe0b687d49afa2f98423d1639cd98b1f010af5e7469dd2c59d4f",
    ),
    (
        ".claude/skills/orchestration-builder/run.md",
        "152e2117e051344130fb8ea1c95ed45801b977f9dcc85e27ecddc88a719e2fc5",
    ),
    (
        ".claude/skills/orchestration-builder/run.md",
        "21c726d2ddd8b98e45fd590de761e2fc416a3a2e2aa3abb91851457a0876c3bd",
    ),
    (
        ".claude/skills/orchestration-builder/run.md",
        "b6d5ab6a25cdc2bbf90648f091563910e8c8e90c9769f9094b119f4c0de83dc6",
    ),
    (
        ".claude/skills/orchestration-builder/stages/1-skills/assign.md",
        "335b091fb82f857a8979c7db74234496f700eafb368fa439af8688b2db475c51",
    ),
    (
        ".claude/skills/orchestration-builder/stages/1-skills/assign.md",
        "8829384696cf04e2f3dd883f7309f76c784b78f3a89c4c93672a868b633b5361",
    ),
    (
        ".claude/skills/orchestration-builder/stages/1-skills/assign.md",
        "e78b70a81110988c58d08cd381fc5ad34c53463b4085f2ce0429fd8825bcb242",
    ),
    (
        ".claude/skills/orchestration-builder/stages/1-skills/stage.md",
        "046dcbfda35634e70f92797847be288d5ed4bc231218f396d0f4b92432377711",
    ),
    (
        ".claude/skills/orchestration-builder/stages/1-skills/stage.md",
        "8e5763ae19ef6bf7666f27f374bdaa26bb6900e32c2c47a0d5cab1a81d4e209b",
    ),
    (
        ".claude/skills/orchestration-builder/stages/1-skills/write-skill.md",
        "21f81d2bcefc7532b6ec8af513c202064b2075305025ae40d83c5f5d31c5facc",
    ),
    (
        ".claude/skills/orchestration-builder/stages/1-skills/write-skill.md",
        "466c566a3549c96f0d1404dc045a51eae219661dad3f7b5ce66a59ca3d966876",
    ),
    (
        ".claude/skills/orchestration-builder/stages/2-work-breakdown/stage.md",
        "076083db2d56f914b1e09bfd15b496ab6d2128b1aa19a7b50e5fb6ffa74b9015",
    ),
    (
        ".claude/skills/orchestration-builder/stages/2-work-breakdown/stage.md",
        "5322cbebdd6fc408c08fe836e80da565b66cbccb9f3fc063e7503f7a3786804e",
    ),
    (
        ".claude/skills/orchestration-builder/stages/3-orchestration/briefs.md",
        "1309564f8348b4a47025685d3b6f6daf2ee96314e97bb12d97b5798bc311bdba",
    ),
    (
        ".claude/skills/orchestration-builder/stages/3-orchestration/briefs.md",
        "a26854c6e2e9245ddca6601b28f4226dc18f630a652f0fb7bb9e1b0e9eac1b7e",
    ),
    (
        ".claude/skills/orchestration-builder/stages/3-orchestration/briefs.md",
        "d1ae0eb9f79497be5b0e2d4b71f35a9e314e8198d7674daf52a613ce52311389",
    ),
    (
        ".claude/skills/orchestration-builder/stages/3-orchestration/build-new.md",
        "150c7c17524831e02629e596411373866b86cf51e1b8bdab494de945edf8a5a0",
    ),
    (
        ".claude/skills/orchestration-builder/stages/3-orchestration/build-new.md",
        "3ecc10ada472571a664be7e3dd8f578a0fcf11153c4bb9064cd34edbad4a32d2",
    ),
    (
        ".claude/skills/orchestration-builder/stages/3-orchestration/build-new.md",
        "48ec4be754a3c8d4f6209adbaabf9f93ba219edb79369b9042313a5d75a10d14",
    ),
    (
        ".claude/skills/orchestration-builder/stages/3-orchestration/limits.md",
        "1d4b35666e875d02dab922d9ccebe40bf9d4690aaa1499d322f5f807913353fa",
    ),
    (
        ".claude/skills/orchestration-builder/stages/3-orchestration/model-function.md",
        "42c97a9cb6b9bed59687b6c4051a982fc3fca515f6555db4040db31c0b7f5981",
    ),
    (
        ".claude/skills/orchestration-builder/stages/3-orchestration/model-function.md",
        "fb3445dc8af8a86216d3bee12dfc07c433308910ada7fcd303d9330ac3798ea8",
    ),
    (
        ".claude/skills/orchestration-builder/stages/3-orchestration/plan-jobs.md",
        "661112bd124e6293b78c38acc4832cc622173ceecdeb009f24903d05f74f3c95",
    ),
    (
        ".claude/skills/orchestration-builder/stages/3-orchestration/plan-jobs.md",
        "825c8eac340a4fbb3b7719601aada0506353e64efb3962f659375869fd6a4215",
    ),
    (
        ".claude/skills/orchestration-builder/stages/3-orchestration/role-skills.md",
        "781dfc8ba5059323c64378f31a91ca6b62834e86f70c1876435fe4e4d331aec4",
    ),
    (
        ".claude/skills/orchestration-builder/stages/3-orchestration/stage.md",
        "611f23622c2c5db2943221b39dd4c5c233d95a4cc5c0cec7463d61575305f567",
    ),
    (
        ".claude/skills/orchestration-builder/stages/3-orchestration/stage.md",
        "f1905c5c0735fef1b2113c3831b4ce0c078bd3e6403e614f95528d526f242742",
    ),
    (
        ".claude/skills/orchestration-builder/stages/challenge/correct.md",
        "c7d08368e633806ea28bfcdfe289abdc6c3c44fb9f8965482eede35bb091eb6c",
    ),
    (
        ".claude/skills/orchestration-builder/stages/challenge/correct.md",
        "d25877d5a41af771b127b4dfb8a961644d179d995f0e7519b9ddd13bbf6f2d3e",
    ),
    (
        ".claude/skills/orchestration-builder/stages/challenge/stage.md",
        "4a465b1cb6f2d7e9492b4f9f644a790243b8f29013d7ced0e2f67298006f67bc",
    ),
    (
        ".claude/skills/orchestration-builder/stages/challenge/stage.md",
        "83cd8d259e0e1f0eec70556bff576b34edd17fe06de343c779c6c6148f6e0834",
    ),
    (
        ".claude/skills/orchestration-builder/stages/challenge/start.md",
        "39f9678775fa434e8707ab643edc90d4ca6b5d928e272049eb3b965a68a457df",
    ),
    (
        ".claude/skills/orchestration-builder/stages/challenge/start.md",
        "496e267683e457e144f5a8f148c26d5fc2461acf7b7abe2c80a0e125b144ace0",
    ),
    (
        ".claude/skills/rex-deliver/SKILL.md",
        "bd334e8e460d03a56000d79c81aea647751fa62298b7dbe34534651cc4b215e8",
    ),
    (
        ".claude/skills/rex-deliver/clean-up.md",
        "bb6a206af73545e1b0e6a0b7d89a7b853608095f6d87d4bccb49b70d97724228",
    ),
    (
        ".claude/skills/rex-deliver/end.md",
        "bc3bce9e3c89277887613699e7698bcbae7b5675fe3a5678b806fecb4dfcd366",
    ),
    (
        ".claude/skills/rex-deliver/github-merge.md",
        "427a395fb557067ba624dbaf50253e5a7f00448ecc975402a1590f4c1c8d865c",
    ),
    (
        ".claude/skills/rex-deliver/more-than-one-repo.md",
        "bea77b44fb79d1b7866d19306efdcf3717ed5920ab519ecccbb2dd6efcf828aa",
    ),
    (
        ".claude/skills/rex-deliver/no-remote-or-no-gh.md",
        "8c24e1018372ccd2e15df0710e5d09531a422bcdbbdeae17697c6d6ab2613d83",
    ),
    (
        ".claude/skills/rex-deliver/parallel-writers.md",
        "89e4122f025f7c3e75d02901e0d9ff5ce2838b2a15be114fae780d5d424fe569",
    ),
    (
        ".claude/skills/rex-deliver/parallel-writers.md",
        "e3212a8cfff6bb6711dfbf40d517eb7a4f893fafc7904ab5123d4b6502e5b620",
    ),
    (
        ".claude/skills/rex-deliver/parallel-writers.md",
        "f02a349a73db15f88058c57da8c81409cb323930b6d3f7991d00165a5674ed01",
    ),
    (
        ".claude/skills/rex-deliver/start.md",
        "79d23282f309ee505b161b9ddfe042586ce915717af5e03dd0e6b3914a39debe",
    ),
    (
        ".claude/skills/rex-deliver/stopped-or-blocked-job.md",
        "4479fc7c52dad1c0b1ad6ef4294666f1b4dabfbaa32c16ab79783dfef2b33701",
    ),
    (
        ".claude/skills/rex-deliver/stopped-or-blocked-job.md",
        "65832115a21e8dbc96d6c250b1be837308113eac3acb6d5551c582146a4edede",
    ),
    (
        ".claude/skills/rex-feedback/SKILL.md",
        "03e046cccbf5b2eb0d2a7187839a95ffc7e4073211a4aa714d0e368d73102d4f",
    ),
    (
        ".claude/skills/rex-feedback/SKILL.md",
        "7f019c55ddd631528b8f758834f401abf42071ec70046aa21da001325ee75593",
    ),
    (
        ".claude/skills/rex-feedback/cause.md",
        "228a98ccb9795851c8c3001a801353083b53a5c8e940a90b353ca15c8d2af789",
    ),
    (
        ".claude/skills/rex-feedback/cause.md",
        "6ce98861f3e776b4cd50fbe3d82ddb4ee6297feb9236572a8ee1b8dbe0b2a81e",
    ),
    (
        ".claude/skills/rex-feedback/change.md",
        "69fe4e8dc38e885845b53a47fbfea1239e1e441728d3b9392996a64bd2045c61",
    ),
    (
        ".claude/skills/rex-feedback/find-job.md",
        "75e102ff0b5cd3e75bfce8457643739d17f8dda6913bee6d28087a692f36ef8d",
    ),
    (
        ".claude/skills/rex-feedback/lesson.md",
        "cf4b7b57d6bb83f92863c1935d05eb36f89df56708c90c34c1d565f996155406",
    ),
    (
        ".claude/skills/rex-grill-me/SKILL.md",
        "63f8a8054c32282547f5f0eea0caedb9eb505bf62e2238294ffd68c5441fc13b",
    ),
    (
        ".claude/skills/rex-pr-review/SKILL.md",
        "39d1f5880b78d6825c071091398b78d2fcb6f27c447bd1a09be207d2fd210ba7",
    ),
    (
        ".claude/skills/rex-pr-review/SKILL.md",
        "69f9b67272d21146341f21a037a2088edfb47c12e014a12939f6aa286218eb59",
    ),
    (
        ".claude/skills/rex-rules/SKILL.md",
        "17b5e28eb69e4a929412c9906b3bc92826cd2782689268566f1fc8b48994dfda",
    ),
    (
        ".claude/skills/rex-rules/SKILL.md",
        "db6c958052d50b702fa7f51b8961d7bacedbef316c26f8568fe834b06e0b4608",
    ),
    (
        ".claude/skills/rex-rules/SKILL.md",
        "f81dfcf78f86f2781e8ea20dde7f84cf9f76462c4c073f01f1692877d18e67d5",
    ),
    (
        ".rex/skills/rex-agent-prompting/SKILL.md",
        "ecc19028ba1fdb0b8868ddde23e4fbd9f79649979d35e6dea6cc3b1185dce2c6",
    ),
    (
        ".rex/skills/rex-agent-prompting/fable.md",
        "41ef73217d3bfb369f49d951a53e5700e73048c2b793cb80a4b5d768a41931e8",
    ),
    (
        ".rex/skills/rex-agent-prompting/haiku.md",
        "91ea13e5614b0af6c3413c055777d705f7d3c0ec5b61d5a59e88bc4397d33141",
    ),
    (
        ".rex/skills/rex-agent-prompting/opus.md",
        "e173018bb30d72401a798881e0a6b8fcbda72824b12fea22ce74062e4dfc0991",
    ),
    (
        ".rex/skills/rex-agent-prompting/sonnet.md",
        "f16f296691db60953bac64120e6afa80618a7c066d62c36796c3c0e6bd939257",
    ),
    (
        ".rex/skills/rex-code-commenting/SKILL.md",
        "8fe79f4b0d7dbb9cc60cd404914faa49cfe0befef0df8a20e6c534c4ac8a2e69",
    ),
    (
        ".rex/skills/rex-code-commenting/examples.md",
        "e90de73195fc732a17b990a81289e2e1ae59a2b3f6a62fdc458872f41ae7edb7",
    ),
    (
        ".rex/skills/rex-code-commenting/module-comments.md",
        "f5d8c70d74d8c6ba6cd56bc0131c5ac61f3753a24f73f1221d7426d6d65e3467",
    ),
    (
        ".rex/skills/rex-code-design-sketch/SKILL.md",
        "26a0e4dae620f8e0a4a165f32892ac0ca09139238633fac9bfbdb0046dd8dffe",
    ),
    (
        ".rex/skills/rex-code-ergonomics/SKILL.md",
        "6e828ffbeb5ee6e13a2db1e7a62c2955ade4eb8558d1ad273744ee66a8f30713",
    ),
    (
        ".rex/skills/rex-code-ergonomics/examples.md",
        "dbadab6ded770f9a93034c28a2f5d947dbd0f3b30bbb4442011be4ae8b8e8db7",
    ),
    (
        ".rex/skills/rex-code-error-writing/SKILL.md",
        "5d8661aa524a2e07013dcd5ff020f420a91b8e0811d3699d12f05d09560130e8",
    ),
    (
        ".rex/skills/rex-code-error-writing/examples.md",
        "521d9a89634475fc157c3f1ef3ccf863aba39154f1e76b2ba2d4f0f6fc47f99a",
    ),
    (
        ".rex/skills/rex-code-external-libraries/SKILL.md",
        "e4331edde5fa891ce18e8858ee081e149f7c0bae3c990bb6c25908a5f87b290a",
    ),
    (
        ".rex/skills/rex-code-failure-policy/SKILL.md",
        "45933608d5530ccb3e5ce0b00ffbee3784798ff33b547e870c0b257ebe5e4477",
    ),
    (
        ".rex/skills/rex-code-failure-policy/SKILL.md",
        "7b25174291417353aed4ea1393f56f4aa67a9156bb5559548a43b6db949a5359",
    ),
    (
        ".rex/skills/rex-code-failure-policy/audit.md",
        "df3add891a4476bf03b1a0bc553d31d613b4795657b2a39d2ffdc83a11567a07",
    ),
    (
        ".rex/skills/rex-code-failure-policy/failure-table.md",
        "0b514551e3f8449007f476861699fbee2da92ad64356df9fe56fc215df418b14",
    ),
    (
        ".rex/skills/rex-code-folder-management/SKILL.md",
        "6a3ed84ee23ebc9ed218b513b4c9fcd656649e07a901e4c8305c04bdc3e625cf",
    ),
    (
        ".rex/skills/rex-code-folder-management/audit.md",
        "5630eb22e252355b7cac50beb1dbc975ae817d0f18d84333b7fb77cba7171bd6",
    ),
    (
        ".rex/skills/rex-code-folder-management/move.md",
        "dc481b365a7d8d23c192411dffc807378ddd1c37917fa3ef718f219f13d74c2b",
    ),
    (
        ".rex/skills/rex-code-improve-codebase-architecture/DEEPENING.md",
        "16d9012f655d604e53e2875d1f99cf8070af2fc23a697895280ffd196e04a995",
    ),
    (
        ".rex/skills/rex-code-improve-codebase-architecture/DEEPENING.md",
        "209ee67b07b1c0e80a94f14ded9c63b9ad2b50e73198f3b664cd3cb61b5e2407",
    ),
    (
        ".rex/skills/rex-code-improve-codebase-architecture/INTERFACE-DESIGN.md",
        "9cc98e8b27c972f79c7877b35866f1a53e9628a4b86cd4cf47417c8511d8abf6",
    ),
    (
        ".rex/skills/rex-code-improve-codebase-architecture/INTERFACE-DESIGN.md",
        "a8339cab22e03f8aa4e053088432bd2c12b958d7968cdc9fa28286d4afe92b29",
    ),
    (
        ".rex/skills/rex-code-improve-codebase-architecture/LANGUAGE.md",
        "84f83fa74a0997a7ff530f754572974783c934b891e21c40026a07d920506867",
    ),
    (
        ".rex/skills/rex-code-improve-codebase-architecture/SKILL.md",
        "6acd531143e5901958008a124875614231708592ad4799185f8babf51564f02d",
    ),
    (
        ".rex/skills/rex-code-improve-codebase-architecture/SKILL.md",
        "c1e9bc4ef252bcbb381a55813518fa67f54ac63a9ffa84d0cd1a44844bb2c2d2",
    ),
    (
        ".rex/skills/rex-code-improve-codebase-architecture/brief.md",
        "634d56543415ee5ee26d5b1c3f0e905884ce91261b7542c4b3ad9f30b8d5676d",
    ),
    (
        ".rex/skills/rex-code-improve-codebase-architecture/brief.md",
        "ba3c9076bfcf13fb770054204cdf87b4972bb7b33d39087753df68b788a56d33",
    ),
    (
        ".rex/skills/rex-code-improve-codebase-architecture/explore.md",
        "78bf9e53099d16469d8a96725f8859c7433cbc45220080505f681fdd8e7236c9",
    ),
    (
        ".rex/skills/rex-code-improve-codebase-architecture/explore.md",
        "ec624ca963ae0bb4c5cd25899edcefda4873e94ca0711d85d776742c75a150c8",
    ),
    (
        ".rex/skills/rex-code-improve-codebase-architecture/grill-self.md",
        "73827e0abb40b9c0991eb2035f474df799f6717e8ce0336c9983ff5090e6f056",
    ),
    (
        ".rex/skills/rex-code-improve-codebase-architecture/grill-self.md",
        "b46a6b886c2b8a99ab77a0bc0e7684b8079b14fa5fea2d81f0d497109ce7872b",
    ),
    (
        ".rex/skills/rex-code-improve-codebase-architecture/rank.md",
        "4967cf65ff553eb85ced603de59ac83ea238c1eca2b128adf5825934db8be419",
    ),
    (
        ".rex/skills/rex-code-improve-codebase-architecture/rank.md",
        "4baf8a77eeb6af83269287474f7ed0e512bdbacd8069f28a6eb362ac92009bc1",
    ),
    (
        ".rex/skills/rex-code-latency/SKILL.md",
        "c0450059fe0625a9c630f9cf778a398f08b517a8a6ade494bac9b298c3a0ebc5",
    ),
    (
        ".rex/skills/rex-code-latency/cpu/branching-code.md",
        "116ee4ee647011d5edeced808f5b1310839d3d3cb4849c9765bf180b9a104a5c",
    ),
    (
        ".rex/skills/rex-code-latency/cpu/branching.md",
        "8d93727f4a1b4b2f106672c64c9c8cd8d9d579e573407372b521b66732e690c5",
    ),
    (
        ".rex/skills/rex-code-latency/cpu/cache-hierarchy-code.md",
        "3a613fc07c27449499df1a8362353df7ca5e85ae074a75014471400a145a2e4b",
    ),
    (
        ".rex/skills/rex-code-latency/cpu/cache-hierarchy.md",
        "8efac2e4c0144fe6202592ee19c96437fbb50ffeb4ed6edf4e6a2fbedf86e7a2",
    ),
    (
        ".rex/skills/rex-code-latency/cpu/cache-line-layout-code.md",
        "aaeb53e94912d9cd0d73ca426d6efcfd02438f433c02eb5e4252cd2161603caa",
    ),
    (
        ".rex/skills/rex-code-latency/cpu/cache-line-layout.md",
        "980d33398ade851f925f258a465ce9fa33bc58d0ace2054aa4b1cd4b01d9077e",
    ),
    (
        ".rex/skills/rex-code-latency/crates.md",
        "40684c28759b296ee028ecb7be47e8679293d7bc5e4c2259991027ab3a05afbe",
    ),
    (
        ".rex/skills/rex-code-latency/glossary.md",
        "9e4cedc128921c6e0061869fcdcb600e8a38010824267e8fc154fa57a53ea741",
    ),
    (
        ".rex/skills/rex-code-latency/measure-code.md",
        "aa8878038ec4959b84c92563c200db252a16347b23c08add71122bb7bdfd14b7",
    ),
    (
        ".rex/skills/rex-code-latency/measure.md",
        "c679e3085ff207ef89d25cfe1baf5cecbfacd5611814da81dc8d223ad0c316e8",
    ),
    (
        ".rex/skills/rex-code-latency/measurement/counters-code.md",
        "fa6687197d7aaf391b7dd673f126d92b5628743427cd9dffa0e4403254428d1f",
    ),
    (
        ".rex/skills/rex-code-latency/measurement/counters.md",
        "6a3a37cd574f83ce3df1c119f89bc6bb21e8427308ca4d564df7332365544335",
    ),
    (
        ".rex/skills/rex-code-latency/measurement/logging-code.md",
        "9b23cee7c482cfa2474ba70a5990c2cbf62a02db7bfc27e21780cd9382f18da0",
    ),
    (
        ".rex/skills/rex-code-latency/measurement/logging.md",
        "7da5c81707a0251b7b917a75e4a033632f2d352760f88bb2c8cf9d11b58584d6",
    ),
    (
        ".rex/skills/rex-code-latency/memory/reclamation-code.md",
        "bd2ed30771343f03d4ff03134eda46218d6f9c85cc2108050981827988c02f90",
    ),
    (
        ".rex/skills/rex-code-latency/memory/reclamation.md",
        "dc30da4c717b98fcfdfd7431e434bd4821bf7c88b0ca670118efbac75f9b954f",
    ),
    (
        ".rex/skills/rex-code-latency/memory/zero-alloc-code.md",
        "c4dc6746c1584c6590e25d24871264db9b04d06d6abcebfb18d8b25333ad83b7",
    ),
    (
        ".rex/skills/rex-code-latency/memory/zero-alloc.md",
        "eb91c2fa0839cd98bf0af4c3a809924644b52110727d4fd4a4b54b2b63fe9cad",
    ),
    (
        ".rex/skills/rex-code-latency/network/kernel-bypass-code.md",
        "74177edef1d2664bd1ec4ada2fe5ed2dc4b849a5b571c94e76fbed8a76f7ae38",
    ),
    (
        ".rex/skills/rex-code-latency/network/kernel-bypass.md",
        "00fbb4291f1495e95219188f03156e86747d2823509cb3e809f592c896683112",
    ),
    (
        ".rex/skills/rex-code-latency/network/sockets-code.md",
        "c8ea3da456c855f9e9169cb9ffdce9c79f177c969f6b52e2dfaa2dd6948c4421",
    ),
    (
        ".rex/skills/rex-code-latency/network/sockets.md",
        "efb2deb1e314409f979d32a0758f7a9fac5d4fc6cdb59423263321ce568d8312",
    ),
    (
        ".rex/skills/rex-code-latency/os/cpu-isolation-code.md",
        "72cfa1698d32f8f01568eeb541fe1f20fc8dc6640acdda92b7b10ab0e55a680e",
    ),
    (
        ".rex/skills/rex-code-latency/os/cpu-isolation.md",
        "d5de84b75735856d5b95b033cfba2a57e55ef8608c187b9e530e95c5fb891331",
    ),
    (
        ".rex/skills/rex-code-latency/os/huge-pages-code.md",
        "8a3223c1ddfa5899407757774feb6fd710fd5fb94e2c3bd1622625a9e294bb0f",
    ),
    (
        ".rex/skills/rex-code-latency/os/huge-pages.md",
        "b10e7400e86956a560b7e77b716c1c45cf77d7a246ef055066b5c213f664db63",
    ),
    (
        ".rex/skills/rex-code-latency/os/jitter.md",
        "74c3297fee2e95bb73e19a7c9eafa239ac6f4be26c0b6154e759b0fa6efacb09",
    ),
    (
        ".rex/skills/rex-code-latency/setup.md",
        "a4a9a00cb21cb5d935faff3fb3d17a4071a3cd17c1a6b3b25b82ef2e9ba22fba",
    ),
    (
        ".rex/skills/rex-code-latency/threads/atomics-code.md",
        "78d1f938e9df7ed84ec3d39ee768365bbcc16b3ebbd7bb32ca6f3e23d6e8b0e6",
    ),
    (
        ".rex/skills/rex-code-latency/threads/atomics.md",
        "93f313afd7d17b7d77abd12a2fcbcb31d9f3ac161fcbb3a8408762427ffce7a4",
    ),
    (
        ".rex/skills/rex-code-latency/threads/ring-buffers-code.md",
        "1698a74bdff6080a3b82112628eb596deaecf48e50e7a42ad4141e695b097f5d",
    ),
    (
        ".rex/skills/rex-code-latency/threads/ring-buffers.md",
        "11d9ebc4928acbe4c26cbd43b96c7dbce0385192055fb2aa313ef93794bca328",
    ),
    (
        ".rex/skills/rex-code-philosophy/SKILL.md",
        "30a6652ddfa5eae6bda41aeb0fb3172b49cb0b7065391ef0cf980f1f3a74875b",
    ),
    (
        ".rex/skills/rex-code-priorities/SKILL.md",
        "2d52c8eab4a1acf70431f1b688cce906852c2f379e2c7385bdef027d66442dac",
    ),
    (
        ".rex/skills/rex-code-priorities/write.md",
        "065f84066117f1ff4b97d17b091b8e7f906a42975b4a2c4d048176bdbd2c6090",
    ),
    (
        ".rex/skills/rex-code-smells/SKILL.md",
        "88f4fbfc443826ae5c49132c1bf76ed20cda4bc287a69d3b258261b7bf5d2360",
    ),
    (
        ".rex/skills/rex-code-smells/brittleness.md",
        "924fadc9ecfbc266c5cb0b0f38d3db88fb09844cf00cba42752ca002eded3c60",
    ),
    (
        ".rex/skills/rex-code-smells/naming.md",
        "577da31f9f51a0ed8d7a75efea322e7e9834a5307f6db980344f71cd96a12635",
    ),
    (
        ".rex/skills/rex-code-smells/rot.md",
        "9b815305c128a2b2187bbb4f3a19ee5643eb7b48a32bda9fd5668bf0c8a0d561",
    ),
    (
        ".rex/skills/rex-code-smells/smells.md",
        "53be06541fd2ccb3e4104c05c6c70ff224b4eb6d52c6b15c6f21845053d4eea8",
    ),
    (
        ".rex/skills/rex-code-smells/sweep.md",
        "d538667bfd28c6a7d9e0725616d13231f0b3c2694c6d497de8e7199377c4038d",
    ),
    (
        ".rex/skills/rex-code-spike/SKILL.md",
        "f93d5620e72077242bf82b5bedc5a3654cf5635299fd6dfe798f278bbc820089",
    ),
    (
        ".rex/skills/rex-code-tdd/SKILL.md",
        "d313c9aa621190f6632e7d131db67cd0f221b50a98d002d0f5d6a3de15a60da4",
    ),
    (
        ".rex/skills/rex-code-tdd/deep-modules.md",
        "a34d65325877cca951aa9c9608a83426ef701fd5f5a8b1dd0f9ecc2861728780",
    ),
    (
        ".rex/skills/rex-code-tdd/interface-design.md",
        "9579dbaae92c3ee636b9330c435c0c212f1b15d8c4aa4f6ccc7e1e340fb5832e",
    ),
    (
        ".rex/skills/rex-code-tdd/mocking.md",
        "621a5210fb8298849327d1edc23477f8c8e8ac6b400d9fc5ad2e9741e946a7f0",
    ),
    (
        ".rex/skills/rex-code-tdd/refactoring.md",
        "02a6d6d283828dfb879eb43a3a424359482cf147bbc7a947a38d01865834d008",
    ),
    (
        ".rex/skills/rex-code-tdd/tests.md",
        "cd4ae9fc4d37b83f2f23c14a50829b7830fbfbbddf1d2d3e0686bba245c76696",
    ),
    (
        ".rex/skills/rex-code-tests-contract-seams/SKILL.md",
        "c645b1e179a2b5594f842d0593fd98fae8db87a90249275a9beead4a82ad2ffe",
    ),
    (
        ".rex/skills/rex-code-tests-contract-seams/examples.md",
        "ee3ff403be6c67ccbb4546eae231d2ef9b792fb6c8ecdbae2425b09e26ea3ee8",
    ),
    (
        ".rex/skills/rex-code-tests-fitness-functions/SKILL.md",
        "c59a7141e22f9d45a5e4ddb497db2636ff3df371b2145069075de2a508f9d30a",
    ),
    (
        ".rex/skills/rex-code-tests-fitness-functions/examples.md",
        "4a8aa08a5882a2b30bf60af6008da449fc16807bb6399848567708506c9c577d",
    ),
    (
        ".rex/skills/rex-code-tests-integration-testing/SKILL.md",
        "a1af55d7e3da661b2382814b578297731a2f83a98e53fde9a79ee0dff8ee4680",
    ),
    (
        ".rex/skills/rex-code-tests-integration-testing/examples.md",
        "29432190758fd871b28b151771236d3f72346e15c2de7f98fbba5aec3f2e6c3d",
    ),
    (
        ".rex/skills/rex-code-tests-integration-testing/real-funds.md",
        "3d2397e6a24edc7791f0ccc423f650514b82c9bb8c519a4be244d83dc5fe0ea5",
    ),
    (
        ".rex/skills/rex-code-tests-unit-testing/SKILL.md",
        "196f1d699542497d1b94c4b8bdd532c3d5d6622613544882607469b8d1587ef5",
    ),
    (
        ".rex/skills/rex-code-validation-plan/SKILL.md",
        "b04147fe5726e4dfbe7aac22083bb31230764dfbbfa6413c4dc193f6c22f25dd",
    ),
    (
        ".rex/skills/rex-code-validation-plan/examples.md",
        "0ae94600784df5ead86e079c095e7e817ff5a0195303fdd186e0ddc6ba8df743",
    ),
    (
        ".rex/skills/rex-codex-adversarial-review/SKILL.md",
        "2bf3584ecde2da999127ed0012464f231db6cd81720e28b632ca77cd57e673e0",
    ),
    (
        ".rex/skills/rex-codex-adversarial-review/SKILL.md",
        "8d5c92b1ea12a04cbf8506ce44581bb3d17c8c6b3b4fd5b41787e8ccccb5ec1e",
    ),
    (
        ".rex/skills/rex-codex-adversarial-review/SKILL.md",
        "98b09ee4d9f17b9c4cde9c87b8e94cea70c691107e179bb997b3d0ae628e4a11",
    ),
    (
        ".rex/skills/rex-mermaid-diagrams/SKILL.md",
        "4ba3c95df47b5c75297dd0705a3b6e14941e1afcc4cd9bc32d370baae0ce07c3",
    ),
    (
        ".rex/skills/rex-mermaid-diagrams/references/advanced-features.md",
        "2e581c7c7a1356837cb5c798406ff7eea32a92304b936590d90414a2924d10f6",
    ),
    (
        ".rex/skills/rex-mermaid-diagrams/references/architecture-diagrams.md",
        "aaec01c3ee975723227c01d876f5f082697d4ff91a9a2d35dd46879dabf2336d",
    ),
    (
        ".rex/skills/rex-mermaid-diagrams/references/c4-diagrams.md",
        "2c2a9392f456a32e5dde6c0cd31ec4254349ea9f1baf70d25c89702fbd3e069f",
    ),
    (
        ".rex/skills/rex-mermaid-diagrams/references/class-diagrams.md",
        "f3ea78df62a434ff9b5e613eb1236d5b0583f26f9606c910f397687822c5a44a",
    ),
    (
        ".rex/skills/rex-mermaid-diagrams/references/erd-diagrams.md",
        "bf657509489b46f7a1be245fbebf9630adca80d1fa20d1b4571a1209b6abfe40",
    ),
    (
        ".rex/skills/rex-mermaid-diagrams/references/flowcharts.md",
        "1485710b847a34a1a8adceacb51ba597cfe4239524a3ebb8dd387174e6a26758",
    ),
    (
        ".rex/skills/rex-mermaid-diagrams/references/sequence-diagrams.md",
        "60295d1611026c287f72d6665a07cce209f89237acf7cdab8d2d7f2191e926dd",
    ),
    (
        ".rex/skills/rex-skill-language/SKILL.md",
        "e02a8d5481b56aec69bc0f0e9914cfbcc1083e7b762f6b53e184697909332a80",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_sorted_so_a_lookup_finds_every_row() {
        assert!(OLD_FILES.is_sorted(), "sort OLD_FILES");
        for (path, hash) in OLD_FILES {
            assert!(
                hash.len() == 64
                    && hash
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
                "{path}: {hash}"
            );
        }
    }

    // A copy of something the plugin no longer ships hides nothing, so this
    // module leaves it alone; a row for one would be dead.
    #[test]
    fn every_old_copy_is_still_shipped_under_its_name() {
        for (path, _) in OLD_FILES {
            let root = OLD_ROOTS
                .iter()
                .find(|root| path.starts_with(&format!("{}/", root.folder)))
                .unwrap_or_else(|| panic!("{path} is under no old root"));
            let rest = &path[root.folder.len() + 1..];
            let name = rest.split('/').next().unwrap_or_default();
            assert!(
                shipped_names(root.shipped).contains(name),
                "{name} is not in {} any more",
                root.shipped
            );
        }
    }
}
