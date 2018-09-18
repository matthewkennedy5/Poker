from enum import Enum
import poker_utils
from poker_utils import Card, Hand, Rank, Suit, HandType


TEST_MODE = True
MINIMUM_BET = 20


class GameStages(Enum):
    PREFLOP = 0
    FLOP = 1
    TURN = 2
    RIVER = 3
    GAME_OVER = 4


class GameState:

    def __init__(self, num_opponents):
        self.stage = GameStages.PREFLOP
        self.num_opponents = num_opponents
        self.pot = 0
        self.read_seat()
        self.read_players()

    def read_seat(self):
        if (TEST_MODE):
            self.seat = 0
        elif self.num_opponents == 1:
            # If it's a 1v1 poker match, there is nobody between you and the 
            # dealer.
            self.seat = 0
        else:
            while True:
                # TODO: Make this input better
                self.seat = int(input("How many players are there between you and the dealer? "))
                if self.seat >= self.num_opponents:
                    print("That's too many.")
                else:
                    break

    # TODO: Make this remember the opponents' names so you don't
    # have to keep typing them in.
    # Sets this.playerNames to the opponent's names.
    # this.playerNames is of length this.numOpponents.
    def read_players(self):
        if TEST_MODE:
            self.player_names = ["Marge", "Barge", "Farge", "Sarge"]
        else:
            self.player_names = []
            print("Enter your opponents' names.")
            for i in range(self.num_opponents):
                name = input('> ')
                self.player_names += name

    def game_is_over(self):
        return self.stage == GameStages.GAME_OVER

    def play_turn(self):
        if TEST_MODE:
            self.turn = Card(Suit.CLUBS, Rank.FOUR)
        else:
            print('Input the turn card.')
            self.turn = read_card()

    def play_river(self):
        if TEST_MODE:
            self.river = Card(Suit.DIAMONDS, Rank.ACE)
        else:
            print('Input the river.')
            self.river = read_card()

    def play_flop(self):
        self.flop = []
        if TEST_MODE:
            self.flop.append(Card(Suit.DIAMONDS, Rank.QUEEN))
            self.flop.append(Card(Suit.HEARTS, Rank.KING))
            self.flop.append(Card(Suit.CLUBS, Rank.KING))
        else:
            print('Input the flop cards.')
            print('First card: ')
            self.flop.append(read_card())
            print('Second card: ')
            self.flop.append(read_card())
            print('Third card: ')
            self.flop.append(read_card())

    def preflop(self):
        self.hole = []
        if TEST_MODE:
            self.hole.append(Card(Suit.DIAMONDS, Rank.FOUR))
            self.hole.append(Card(Suit.SPADES, Rank.TWO))
        else:
            print('Input your hold cards.')
            print('Frist card: ')
            self.hole.append(read_card())
            print('Second card: ')
            self.hole.append(read_card())

    def advance(self):
        print()
        if self.stage == GameStages.PREFLOP:
            self.preflop()
            print('Pre-flop: ')
        elif self.stage == GameStages.FLOP:
            print('Flop: ')
            self.play_flop()
        elif self.stage == GameStages.TURN:
            print('Turn: ')
            self.play_turn()
        elif self.stage == GameStages.RIVER:
            print('River: ')
            self.play_river()
        elif self.stage == GameStages.GAME_OVER:
            print('Thanks for playing!')
            return
        self.bet()
        self.stage += 1

    # TODO: add an UNDO capability to all inputs

    def init_player_bets(self):
        result = {}
        for i in range(self.num_opponents):
            result[self.player_names[i]] = 0
        return result

    # // TODO: make playerBets a Map of player name -> bet instead of an array.

    # // TODO: make it keep track of how much money you have 

    def bet(self):
        print()
        ante = 0
        if (self.stage == GameStages.PREFLOP):
            ante = MINIMUM_BET
        my_bet = 0
        player_bets = self.init_player_bets()
        while True:
            # Players before me
            for i in range(self.seat):
                # TODO: add support for player names
                ante = this.process_player_bet(i, ante, player_bets)
            chance_of_winning = self.chance_of_winning()
            need_to_bet = ante - my_bet
            # // TODO: Experiment with what happens to the expected value when it is assumed that nobody folds.
            expected_value = chance_of_winning * self.pot - (1 - chance_of_winning) * need_to_bet
            if TEST_MODE:
                print('Chance of winning: %f' % chance_of_winning)
                print('Expected value: $%f' % expected_value)
            if expected_value < 0 and self.stage != GameStages.PREFLOP:
                self.fold()
                return
            else:
                print("Bet %f" % need_to_bet)
                my_bet += need_to_bet
            # players after me
            for i in range(self.seat, self.num_opponents):
                import pdb; pdb.set_trace()
                ante = self.process_player_bet(i, ante, player_bets)
                print("ante: %f" % ante)
                print("player_bets %f" % player_bets)
            if this.betting_done(player_bets, my_bet):
                break

    # // TODO: make an expected value function (bet to maximize expected value assuming some other players bet too)

    def fold(self):
        print('Fold')
        # Advance the game stage to the end
        self.stage = GameStages.river

    def betting_done(player_bets, my_bet):
        # Betting is over if every bet in playerBets and myBet are equal.
        # START HERE: figure out what the hell is wrong with this code. Also
        for bet in player_bets:
            if bet != my_bet:
                return False
        return True

    # Returns the new ante
    # TODO: Make code use player names instead of numbers
    def process_player_bet(self, name, ante, player_bets):
        if TEST_MODE:
            action = 20
        else:
            action = input("%s's bet: " % name)
        if action == 'fold':
            # self.seat--   TODO: This should happen in some cases. Fix
            self.num_opponents -= 1
        else:
            # TODO: what the hell is action?
            bet = int(action)
            self.pot += bet
            player_bets[name] += bet
            if player_bets[name] > ante:
                ante = player_bets[name]
        return ante

    def preflop_chance_of_winning(self):
        # TODO: rewrite in a non-stupid way
        probability_sum = 0
        num_reps = 0
        for suit3 in Suit:
            for rank3 in Rank:
                card3 = Card(suit3, rank3)
                if card3 != self.hole[0] and card3 != self.hole[1]:
                    for suit4 in Suit:
                        for rank4 in Rank:
                            card4 = Card(suit4, rank4)
                            if card4 != card3 and card4 != self.hole[1] and card4 != self.hole[0]:
                                 for suit5 in Suit:
                                    for rank5 in Rank:
                                        card5 = Card(suit5, rank5)
                                        if card5 != card4 and card5 != card3 and card5 != self.hole[1] and card5 !=self.hole[0]:
                                            opponent_hand = Hand([self.hole[0], self.hole[1], card3, card4, card5])
                                            assert opponent_hand.chance_of_winning(self.num_opponents) <= 1  # TODO: remove
                                            probability_sum += opponent_hand.chance_of_winning(self.num_opponents)
                                            num_reps += 1
        average_probability = probability_sum / (50 * 49 * 48)  # TODO: Fix magic numbers
        return average_probability

    def get_hand(self):
        if self.stage == GameStages.PREFLOP:
            raise Exception('No hand yet')  # TODO: specify which type of exception
        elif self.stage == GameStages.FLOP:
            return Hand(self.hole + self.flop)
        elif self.stage == GameStages.TURN:
            return best_hand(self.hole + self.flop + self.turn)
        elif self.stage == GameStages.RIVER:
            return best_hand(self.hole + self.flop + self.turn + self.river)
        else:
            raise Exception('whoops')   # TODO: better exception handling

    def opponent_chance_of_winning(self):
        num_opponent_wins = 0
        num_iterations = 0
        deck = helpers.get_deck()
        for i in range(len(deck)):
            opponent_hole1 = deck[i]
            if self.card_is_unique(opponent_hole1):
                for j in range(len(deck)):
                    opponent_hole2 = deck[j]
                    if self.card_is_unique(opponent_hole2) and opponent_hole1 != opponent_hole2:
                        hand = best_hand(self.flop + self.turn + self.river + opponent_hole1 + opponent_hole2)
                        if hand > self.get_hand():
                            num_opponent_wins += 1
                        num_iterations += 1
        return num_opponent_wins / num_iterations

    def chance_of_winning(self):
        if self.stage == GameStages.PREFLOP:
            return self.preflop_chance_of_winning()
        chance_of_beating_one_opponent = 1 - self.opponent_chance_of_winning()
        return chance_of_beating_one_opponent ** self.num_opponents

    def card_is_unique(self, card):
        # Check my hole cards
        for hole_card in self.hole:
            if card == hole_card:
                return False
        # Check the flop
        for flop_card in self.flop:
            if card == flop_card:
                return False
        # Check the turn
        if self.turn is not None and card == self.turn:
            return False
        return True

            

