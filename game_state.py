from enum import IntEnum
import poker_utils
from poker_utils import Card, Hand, Rank, Suit, HandType, best_hand, read_card
import pdb


TEST_MODE = False
MINIMUM_BET = 10
MAXIMUM_BET = 100


class GameStages(IntEnum):
    PREFLOP = 0
    FLOP = 1
    TURN = 2
    RIVER = 3
    GAME_OVER = 4

DEALER = -1

class GameState:

    def __init__(self, num_opponents):
        self.stage = GameStages.PREFLOP
        self.num_opponents = num_opponents
        self.pot = 0
        self.hole = None
        self.turn = None
        self.river = None
        self.read_seat()
        self.read_players()

    def read_seat(self):
        if (TEST_MODE):
            self.seat = 0
        elif self.num_opponents == 1:
            # If it's a 1v1 poker match, there is nobody between you and the 
            # dealer.
            dealer = input('Are you the dealer [y/n]? ').lower()
            if dealer == 'y' or dealer == 'yes':
                self.seat = DEALER
            else:
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
    # Sets self.playerNames to the opponent's names.
    # this.playerNames is of length this.numOpponents.
    def read_players(self):
        if TEST_MODE:
            self.player_names = ["Marge", "Barge", "Farge", "Sarge"]
        else:
            self.player_names = []
            print("Enter your opponents' names.")
            for i in range(self.num_opponents):
                name = input('> ')
                self.player_names.append(name)

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
            print('First card: ')
            self.hole.append(read_card())
            print('Second card: ')
            self.hole.append(read_card())

    def advance(self):
        print()
        if self.stage == GameStages.PREFLOP:
            print('Pre-flop: ')
            self.preflop()
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
    # TODO: Make a GUI

    # TODO: Make player bets be ints, not doubles
    def init_player_bets(self):
        result = {}
        for i in range(self.num_opponents):
            result[self.player_names[i]] = 0
        return result

    # // TODO: make it keep track of how much money you have 

    def _opponent_expected_value(self):
        # Iterate over opponent's possible hands
        # See how many beat me
        # Find optimal opponent bet given my bet


    def _expected_value(self):
        if self.seat != DEALER:
            expected_opponent_bet = 10  # TODO: Implement an opponent expected value function
        else:
            chance_of_winning = self._chance_of_winning()
            return chance_of_winning * self.pot - (1 - chance_of_winning) * need_to_bet

    def _opponent_optimal_bet(self):

    def _optimal_bet(self, need_to_bet):
        learning_rate = 1
        optimal_bet = need_to_bet
        chance_of_winning = self._chance_of_winning()
        for i in range(100):
            expected_opponent_bet = self._opponent_optimal_bet(my_bet)
            







    def bet(self):
        print()
        ante = 0
        if (self.stage == GameStages.PREFLOP):
            ante = MINIMUM_BET
        my_bet = 0
        player_bets = self.init_player_bets()
        while True:
            if self.seat == DEALER:
                ante = self.process_player_bet(self.player_names[0], ante, player_bets)
            need_to_bet = ante - my_bet
            # // TODO: Experiment with what happens to the expected value when it is assumed that nobody folds.
            expected_value = self._expected_value()
            if True:
                print('Chance of winning: %f' % chance_of_winning)
                print('Expected value: $%.2f' % expected_value)
            if expected_value < 0:
                self.fold()
                return
            else:
                # TODO: Add the option to raise the bet to maximize expected value
                print("Bet $%.2f" % need_to_bet)
                my_bet += need_to_bet
                if my_bet < MINIMUM_BET:
                    my_bet = MINIMUM_BET
            if self.seat != DEALER:
                name = self.player_names[0]
                ante = self.process_player_bet(name, ante, player_bets)
            if self.betting_done(player_bets, my_bet):
                break

    # // TODO: make an expected value function (bet to maximize expected value assuming some other players bet too)

    def fold(self):
        print('Fold')
        # Advance the game stage to the end
        self.stage = GameStages.river

    def betting_done(self, player_bets, my_bet):
        # Betting is over if every bet in playerBets and myBet are equal.
        for bet in player_bets.values():
            if bet != my_bet:
                return False
        return True

    # Returns the new ante
    def process_player_bet(self, name, ante, player_bets):
        if TEST_MODE:
            action = 20
        else:
            action = input("%s's bet: $" % name)
        if action == 'fold':
            # self.seat--   TODO: This should happen in some cases. Fix
            self.num_opponents -= 1
        else:
            # TODO: Add 'check' option
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
                                            try:
                                                probability_sum += opponent_hand.chance_of_winning(self.num_opponents)
                                            except:
                                                import pdb; pdb.set_trace()
                                            num_reps += 1
        average_probability = probability_sum / (50 * 49 * 48)  # TODO: Fix magic numbers
        return average_probability

    def get_hand(self):
        if self.stage == GameStages.PREFLOP:
            raise Exception('No hand yet')  # TODO: specify which type of exception
        elif self.stage == GameStages.FLOP:
            return Hand(self.hole + self.flop)
        elif self.stage == GameStages.TURN:
            return best_hand(self.hole + self.flop + [self.turn])
        elif self.stage == GameStages.RIVER:
            return best_hand(self.hole + self.flop + [self.turn, self.river])
        else:
            raise Exception('whoops')   # TODO: better exception handling

    def opponent_chance_of_winning(self):
        # TODO: Rewrite using itertools combinations
        num_opponent_wins = 0
        num_iterations = 0
        deck = poker_utils.get_deck()
        for i in range(len(deck)):
            opponent_hole1 = deck[i]
            if self.card_is_unique(opponent_hole1):
                for j in range(len(deck)):
                    opponent_hole2 = deck[j]
                    if self.card_is_unique(opponent_hole2) and opponent_hole1 != opponent_hole2:
                        opponent_cards = self.flop + [opponent_hole1, opponent_hole2]
                        if self.turn is not None:
                            opponent_cards.append(self.turn)
                            if self.river is not None:
                                opponent_cards.append(self.river)
                        hand = best_hand(opponent_cards)
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

            

