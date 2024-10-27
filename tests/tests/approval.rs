use crate::init::init;

use near_contract_standards::non_fungible_token::Token;
use near_workspaces::{types::NearToken, AccountId};

use std::collections::HashMap;
use std::convert::TryFrom;

pub const TOKEN_ID: &str = "0";

const ONE_NEAR: NearToken = NearToken::from_near(1);
const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);

#[tokio::test]
async fn test_simple_approve() -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let (nft_contract, _, token_receiver_contract, alice) = init(&worker).await?;

    // root approves alice
    let res = nft_contract
        .call("nft_approve")
        .args_json((TOKEN_ID, alice.id(), Option::<String>::None))
        .max_gas()
        .deposit(NearToken::from_yoctonear(510000000000000000000))
        .transact()
        .await?;
    assert!(res.is_success());

    // check nft_is_approved, don't provide approval_id
    let alice_approved = nft_contract
        .call("nft_is_approved")
        .args_json((TOKEN_ID, alice.id(), Option::<u64>::None))
        .view()
        .await?
        .json::<bool>()?;
    assert!(alice_approved);

    // check nft_is_approved, with approval_id=1
    let alice_approval_id_is_1 = nft_contract
        .call("nft_is_approved")
        .args_json((TOKEN_ID, alice.id(), Some(1u64)))
        .view()
        .await?
        .json::<bool>()?;
    assert!(alice_approval_id_is_1);

    // check nft_is_approved, with approval_id=2
    let alice_approval_id_is_2 = nft_contract
        .call("nft_is_approved")
        .args_json(&(TOKEN_ID, alice.id(), Some(2u64)))
        .view()
        .await?
        .json::<bool>()?;
    assert!(!alice_approval_id_is_2);

    // alternatively, one could check the data returned by nft_token
    let token = nft_contract
        .call("nft_token")
        .args_json((TOKEN_ID,))
        .view()
        .await?
        .json::<Token>()?;
    let mut expected_approvals: HashMap<AccountId, u64> = HashMap::new();
    expected_approvals.insert(AccountId::try_from(alice.id().to_string())?, 1);
    assert_eq!(token.approved_account_ids.unwrap(), expected_approvals);

    // root approves alice again, which changes the approval_id and doesn't require as much deposit
    let res = nft_contract
        .call("nft_approve")
        .args_json((TOKEN_ID, alice.id(), Option::<String>::None))
        .max_gas()
        .deposit(ONE_NEAR)
        .transact()
        .await?;
    assert!(res.is_success());

    let alice_approval_id_is_2 = nft_contract
        .call("nft_is_approved")
        .args_json((TOKEN_ID, alice.id(), Some(2u64)))
        .view()
        .await?
        .json::<bool>()?;
    assert!(alice_approval_id_is_2);

    // approving another account gives different approval_id
    let res = nft_contract
        .call("nft_approve")
        .args_json((
            TOKEN_ID,
            token_receiver_contract.id(),
            Option::<String>::None,
        ))
        .max_gas()
        .deposit(NearToken::from_yoctonear(510000000000000000000))
        .transact()
        .await?;
    assert!(res.is_success());

    let token_receiver_approval_id_is_3 = nft_contract
        .call("nft_is_approved")
        .args_json((TOKEN_ID, token_receiver_contract.id(), Some(3u64)))
        .view()
        .await?
        .json::<bool>()?;
    assert!(token_receiver_approval_id_is_3);

    Ok(())
}

#[tokio::test]
async fn test_approval_with_call() -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let (nft_contract, approval_receiver_contract, _, _) = init(&worker).await?;

    let res = nft_contract
        .call("nft_approve")
        .args_json((
            TOKEN_ID,
            approval_receiver_contract.id(),
            Some("return-now".to_string()),
        ))
        .max_gas()
        .deposit(NearToken::from_yoctonear(450000000000000000000))
        .transact()
        .await?;
    assert_eq!(res.json::<String>()?, "cool".to_string());

    // Approve again; will set different approval_id (ignored by approval_receiver).
    // The approval_receiver implementation will return given `msg` after subsequent promise call,
    // if given something other than "return-now".
    let msg = "hahaha".to_string();
    let res = nft_contract
        .call("nft_approve")
        .args_json((TOKEN_ID, approval_receiver_contract.id(), Some(msg.clone())))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert_eq!(res.json::<String>()?, msg);

    Ok(())
}

#[tokio::test]
async fn test_approved_account_transfers_token() -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let (nft_contract, _, _, alice) = init(&worker).await?;

    // root approves alice
    let res = nft_contract
        .call("nft_approve")
        .args_json((TOKEN_ID, alice.id(), Option::<String>::None))
        .max_gas()
        .deposit(NearToken::from_yoctonear(510000000000000000000))
        .transact()
        .await?;
    assert!(res.is_success());

    // alice sends to self
    let res = alice
        .call(nft_contract.id(), "nft_transfer")
        .args_json((
            alice.id(),
            TOKEN_ID,
            Some(1u64),
            Some("gotcha! bahahaha".to_string()),
        ))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    // token now owned by alice
    let token = nft_contract
        .call("nft_token")
        .args_json((TOKEN_ID,))
        .view()
        .await?
        .json::<Token>()?;
    assert_eq!(token.owner_id.to_string(), alice.id().to_string());

    Ok(())
}

#[tokio::test]
async fn test_revoke() -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let (nft_contract, _, token_receiver_contract, alice) = init(&worker).await?;

    // root approves alice
    let res = nft_contract
        .call("nft_approve")
        .args_json((TOKEN_ID, alice.id(), Option::<String>::None))
        .max_gas()
        .deposit(NearToken::from_yoctonear(510000000000000000000))
        .transact()
        .await?;
    assert!(res.is_success());

    // root approves token_receiver
    let res = nft_contract
        .call("nft_approve")
        .args_json((
            TOKEN_ID,
            token_receiver_contract.id(),
            Option::<String>::None,
        ))
        .max_gas()
        .deposit(NearToken::from_yoctonear(450000000000000000000))
        .transact()
        .await?;
    assert!(res.is_success());

    // root revokes alice
    let res = nft_contract
        .call("nft_revoke")
        .args_json((TOKEN_ID, alice.id()))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    // alice is revoked...
    let alice_approved = nft_contract
        .call("nft_is_approved")
        .args_json((TOKEN_ID, alice.id(), Some(3u64)))
        .view()
        .await?
        .json::<bool>()?;
    assert!(!alice_approved);

    // but token_receiver is still approved
    let token_receiver_approved = nft_contract
        .call("nft_is_approved")
        .args_json((TOKEN_ID, token_receiver_contract.id(), Option::<u64>::None))
        .view()
        .await?
        .json::<bool>()?;
    assert!(token_receiver_approved);

    // root revokes token_receiver
    let res = nft_contract
        .call("nft_revoke")
        .args_json((TOKEN_ID, token_receiver_contract.id()))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    // alice is still revoked...
    let alice_approved = nft_contract
        .call("nft_is_approved")
        .args_json((TOKEN_ID, alice.id(), Some(3u64)))
        .view()
        .await?
        .json::<bool>()?;
    assert!(!alice_approved);

    // ...and now so is token_receiver
    let token_receiver_approved = nft_contract
        .call("nft_is_approved")
        .args_json((TOKEN_ID, token_receiver_contract.id(), Option::<u64>::None))
        .view()
        .await?
        .json::<bool>()?;
    assert!(!token_receiver_approved);

    Ok(())
}

#[tokio::test]
async fn test_revoke_all() -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox().await?;
    let (nft_contract, _, token_receiver_contract, alice) = init(&worker).await?;

    // root approves alice
    let res = nft_contract
        .call("nft_approve")
        .args_json((TOKEN_ID, alice.id(), Option::<String>::None))
        .max_gas()
        .deposit(NearToken::from_yoctonear(510000000000000000000))
        .transact()
        .await?;
    assert!(res.is_success());

    // root approves token_receiver
    let res = nft_contract
        .call("nft_approve")
        .args_json((
            TOKEN_ID,
            token_receiver_contract.id(),
            Option::<String>::None,
        ))
        .max_gas()
        .deposit(NearToken::from_yoctonear(450000000000000000000))
        .transact()
        .await?;
    assert!(res.is_success());

    // root revokes all
    let res = nft_contract
        .call("nft_revoke_all")
        .args_json((TOKEN_ID,))
        .max_gas()
        .deposit(ONE_YOCTO)
        .transact()
        .await?;
    assert!(res.is_success());

    // alice is revoked...
    let alice_approved = nft_contract
        .call("nft_is_approved")
        .args_json((TOKEN_ID, alice.id(), Some(3u64)))
        .view()
        .await?
        .json::<bool>()?;
    assert!(!alice_approved);

    // and so is token_receiver
    let token_receiver_approved = nft_contract
        .call("nft_is_approved")
        .args_json((TOKEN_ID, token_receiver_contract.id(), Option::<u64>::None))
        .view()
        .await?
        .json::<bool>()?;
    assert!(!token_receiver_approved);

    Ok(())
}
