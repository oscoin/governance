use std::convert::TryFrom;

use librad::git::{
    local::url::LocalUrl,
    types::{remote::LocalPushspec, Fetchspec, Force, Remote},
};
use radicle_git_ext::RefspecPattern;

use coco::{identities::payload::Person, state, RunConfig};

#[macro_use]
mod common;
use common::{build_peer, init_logging, shia_le_pathbuf, started};

#[tokio::test]
async fn merge_request() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    let alice_tmp_dir = tempfile::tempdir()?;
    let alice_repo_path = alice_tmp_dir.path().join("radicle");
    let alice_peer = build_peer(&alice_tmp_dir, RunConfig::default()).await?;
    let (alice_peer, alice_addrs) = {
        let peer = alice_peer.peer.clone();
        let events = alice_peer.subscribe();
        let mut peer_control = alice_peer.control();
        tokio::task::spawn(alice_peer.run());
        started(events).await?;
        let alice_addrs = peer_control.listen_addrs().await;
        (peer, alice_addrs)
    };
    let alice = state::init_owner(
        &alice_peer,
        Person {
            name: "alice".into(),
        },
    )
    .await?;
    let alice_peer_id = alice_peer.peer_id();
    let alice_signature =
        git2::Signature::now(&alice.subject().name.to_string(), "alice@example.com")?;

    let bob_tmp_dir = tempfile::tempdir()?;
    let bob_peer = build_peer(&bob_tmp_dir, RunConfig::default()).await?;
    let bob_peer = {
        let peer = bob_peer.peer.clone();
        let events = bob_peer.subscribe();
        tokio::task::spawn(bob_peer.run());
        started(events).await?;
        peer
    };
    let _bob = state::init_owner(&bob_peer, Person { name: "bob".into() }).await?;

    let project = state::init_project(
        &alice_peer,
        &alice,
        shia_le_pathbuf(alice_repo_path.clone()),
    )
    .await?;

    state::clone_project(
        &bob_peer,
        project.urn(),
        alice_peer_id,
        alice_addrs.clone(),
        None,
    )
    .await?;

    let repo = git2::Repository::open(alice_repo_path.join(project.subject().name.to_string()))?;
    let default_branch = project.subject().default_branch.clone().unwrap();

    let head_commit_id = repo
        .find_reference(&format!("refs/heads/{}", default_branch))?
        .target()
        .unwrap();

    let head_commit = repo.find_object(head_commit_id, None).unwrap();
    let _tag_id = repo
        .tag(
            "radicle-merge-request/MY-MR",
            &head_commit,
            &alice_signature,
            "MESSAGE",
            false,
        )
        .unwrap();

    let mut rad =
        Remote::<LocalUrl>::rad_remote::<_, Fetchspec>(LocalUrl::from(project.urn()), None);
    let _ = rad.push(
        state::settings(&alice_peer),
        &repo,
        LocalPushspec::Matching {
            pattern: RefspecPattern::try_from("refs/tags/*").unwrap(),
            force: Force::False,
        },
    )?;

    // alice sees their own merge request
    let alice_merge_requests = coco::merge_request::list(&alice_peer, project.urn())
        .await
        .unwrap();
    assert_eq!(
        alice_merge_requests.len(),
        1,
        "testing alice's merge request list"
    );
    assert_eq!(
        &alice_merge_requests[0].id, "MY-MR",
        "testing alice's merge request list"
    );

    // bob sees alice's merge request
    state::fetch(&bob_peer, project.urn(), alice_peer_id, alice_addrs, None).await?;

    let bob_merge_requests = coco::merge_request::list(&bob_peer, project.urn())
        .await
        .unwrap();
    assert_eq!(
        bob_merge_requests.len(),
        1,
        "testing bob's merge request list"
    );
    assert_eq!(
        &bob_merge_requests[0].id, "MY-MR",
        "testing bob's merge request list"
    );

    Ok(())
}
