// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {ISP1VerifierGateway} from "@sp1-contracts/ISP1VerifierGateway.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

/// @title SP1 Merkle Root Verifier
/// @notice This contract verifies SP1 proofs and manages a merkle root.
contract SP1AggregateVerifier is Ownable {
    /// @notice The address of the SP1 verifier gateway contract
    ISP1VerifierGateway public verifier;
    /// @notice The verification key for the aggregate program
    bytes32 public immutable PROGRAM_VKEY;
    /// @notice The current merkle root of all verified proofs
    bytes32 public merkleRoot;

    event MerkleRootUpdated(bytes32 oldRoot, bytes32 newRoot);

    constructor(address _verifierGateway, bytes32 _programVKey) Ownable(msg.sender) {
        verifier = ISP1VerifierGateway(_verifierGateway);
        PROGRAM_VKEY = _programVKey;
    }

    /// @notice Verifies an aggregate proof and updates the merkle root
    /// @param _publicValues The new merkle root
    /// @param _proofBytes The encoded aggregate proof
    function verifyAggregateProofAndUpdateRoot(
        bytes calldata _publicValues,
        bytes calldata _proofBytes
    ) public onlyOwner {
        verifier.verifyProof(
            PROGRAM_VKEY,
            _publicValues,
            _proofBytes
        );
        
        bytes32 oldRoot = merkleRoot;
        merkleRoot = abi.decode(_publicValues, (bytes32));
        emit MerkleRootUpdated(oldRoot, merkleRoot);
    }
}
