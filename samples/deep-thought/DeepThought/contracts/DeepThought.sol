// SPDX-License-Identifier: GPL-3.0-or-later
pragma solidity ^0.6.0;

contract DeepThought {
  event GreatOnTurning(address programmers, uint64 thinkingUntil);
  event DayOfTheAnswer(address descendants, uint256 answer);

  uint64 private thinkingUntil;
  bool private answered;

  constructor() public {
    uint256 years_ = 365 days;
    thinkingUntil = uint64(block.timestamp + 7.5e6 * years_);
    answered = false;
    emit GreatOnTurning(msg.sender, thinkingUntil);
  }

  function receiveAnswer() public returns (uint256) {
    require(block.timestamp >= thinkingUntil, "still thinking about it...");
    if (!answered) {
      answered = true;
      emit DayOfTheAnswer(msg.sender, block.timestamp);
    }
    return 42;
  }
}
