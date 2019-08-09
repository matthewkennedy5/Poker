import unittest
import itertools
import random
from tqdm import tqdm, trange
from scipy import stats
from matplotlib import pyplot as plt
from texas_hands import *
from hand_abstraction import *
from texas_utils import *
from hand_table import *
from trainer import *


class HandTests(unittest.TestCase):

    def test_classification(self):
        royal_flush = TexasHand(('As', 'Js', 'Ks', 'Qs', 'Ts'))
        straight_flush = TexasHand(('7d', '8d', 'Jd', '9d', 'Td'))
        four = TexasHand(('2h', '2c', '7d', '2d', '2s'))
        full_house = TexasHand(('As', 'Jd', 'Jc', 'Ac', 'Ah'))
        flush = TexasHand(('Jh', '2h', '3h', '7h', '9h'))
        straight = TexasHand(('Ah', '2s', '3d', '5c', '4c'))
        trips = TexasHand(('5d', '4c', '6d', '6h', '6c'))
        two_pair = TexasHand(('6d', '5c', '5h', 'Ah', 'Ac'))
        pair = TexasHand(('Ah', '2d', '2s', '3c', '5c'))
        high_card = TexasHand(('Kh', 'Ah', 'Qh', '2h', '3s'))
        self.assertEqual(royal_flush.type, ROYAL_FLUSH)
        self.assertEqual(straight_flush.type, STRAIGHT_FLUSH)
        self.assertEqual(four.type, FOUR_OF_A_KIND)
        self.assertEqual(full_house.type, FULL_HOUSE)
        self.assertEqual(flush.type, FLUSH)
        self.assertEqual(straight.type, STRAIGHT)
        self.assertEqual(trips.type, THREE_OF_A_KIND)
        self.assertEqual(two_pair.type, TWO_PAIR)
        self.assertEqual(pair.type, PAIR)
        self.assertEqual(high_card.type, HIGH_CARD)

    def test_six_cards(self):
        royal_flush = TexasHand(('Jd', 'As', 'Js', 'Ks', 'Qs', 'Ts'))
        straight_flush = TexasHand(('7d', '2c', '8d', 'Jd', '9d', 'Td'))
        four = TexasHand(('2h', '2c', '3d', '7d', '2d', '2s'))
        full_house = TexasHand(('As', 'Jd', 'Jc', '2c', 'Ac', 'Ah'))
        flush = TexasHand(('Jh', '2h', '3h', '7h', 'Ts', '9h'))
        straight = TexasHand(('Ah', '2s', '3d', '5c', '4c', 'Td'))
        trips = TexasHand(('Ad', '5d', '4c', '6d', '6h', '6c'))
        two_pair = TexasHand(('6d', '5c', '5h', 'Ah', 'Ac', '2c'))
        pair = TexasHand(('Ah', '2d', '2s', '3c', 'Th', '5c'))
        high_card = TexasHand(('Kh', 'Ah', '9d', 'Qh', '2h', '3s'))
        self.assertEqual(royal_flush.type, ROYAL_FLUSH)
        self.assertEqual(straight_flush.type, STRAIGHT_FLUSH)
        self.assertEqual(four.type, FOUR_OF_A_KIND)
        self.assertEqual(full_house.type, FULL_HOUSE)
        self.assertEqual(flush.type, FLUSH)
        self.assertEqual(straight.type, STRAIGHT)
        self.assertEqual(trips.type, THREE_OF_A_KIND)
        self.assertEqual(two_pair.type, TWO_PAIR)
        self.assertEqual(pair.type, PAIR)
        self.assertEqual(high_card.type, HIGH_CARD)


    def test_seven_cards(self):
        royal_flush = TexasHand(('Jd', 'As', 'Js', 'Ks', 'Qs', 'Ts', '2c'))
        straight_flush = TexasHand(('7d', '2c', '8d', 'Jd', '9d', '3d', 'Td'))
        four = TexasHand(('2h', '2c', '3d', '5c', '7d', '2d', '2s'))
        full_house = TexasHand(('As', 'Jd', 'Qs', 'Jc', '2c', 'Ac', 'Ah'))
        flush = TexasHand(('Jh', '2c', '2h', '3h', '7h', 'Ts', '9h'))
        straight = TexasHand(('2c', 'Ah', '2s', '3d', '5c', '4c', 'Td'))
        trips = TexasHand(('2c', 'Ad', '5d', '4c', '6d', '6h', '6c'))
        two_pair = TexasHand(('6d', '5c', 'Td', '5h', 'Ah', 'Ac', '2c'))
        pair = TexasHand(('Ah', '2d', '2s', '3c', 'Th', '5c', 'Qh'))
        high_card = TexasHand(('Kh', 'Ah', '9d', 'Qh', '2h', '6d', '3s'))
        self.assertEqual(royal_flush.type, ROYAL_FLUSH)
        self.assertEqual(straight_flush.type, STRAIGHT_FLUSH)
        self.assertEqual(four.type, FOUR_OF_A_KIND)
        self.assertEqual(full_house.type, FULL_HOUSE)
        self.assertEqual(flush.type, FLUSH)
        self.assertEqual(straight.type, STRAIGHT)
        self.assertEqual(trips.type, THREE_OF_A_KIND)
        self.assertEqual(two_pair.type, TWO_PAIR)
        self.assertEqual(pair.type, PAIR)
        self.assertEqual(high_card.type, HIGH_CARD)

    # Should raise a value error when incorrect cards are provided.
    def test_errors(self):
        # Invalid card strings
        with self.assertRaises(ValueError):
            TexasHand(('blah', 'nope', '3h', 'foobar', '7d'))
        with self.assertRaises(ValueError):
            TexasHand(('Td', 'Th', 'Tc', '10s', '9d', '4h'))
        # Too many cards
        with self.assertRaises(ValueError):
            TexasHand(('Td', 'Th', 'Tc', '9c', '3h', 'Ts', '9d', '4h'))
        # Too few cards
        with self.assertRaises(ValueError):
            TexasHand(('7c', '7h', '7d', '7s'))
        # Duplicate cards
        with self.assertRaises(ValueError):
            TexasHand(('7c', '7c', '7h', '7d', '7s'))
        # Non-string in list
        with self.assertRaises(TypeError):
            TexasHand((1, 2, 3, 4, 5))

    def test_comparisons(self):
        # TODO: Write more tricky hand comparison tests to make sure it really works.
        royal_flush = TexasHand(('Jd', 'As', 'Js', 'Ks', 'Qs', 'Ts', '2c'))
        straight_flush = TexasHand(('7d', '2c', '8d', 'Jd', '9d', '3d', 'Td'))
        four = TexasHand(('2h', '2c', '3d', '5c', '7d', '2d', '2s'))
        full_house = TexasHand(('As', 'Jd', 'Qs', 'Jc', '2c', 'Ac', 'Ah'))
        same_full_house = TexasHand(('As', 'Js', '2s', 'Jc', '2c', 'Ac', 'Ah'))
        flush = TexasHand(('Jh', '2c', '2h', '3h', '7h', 'As', '9h'))
        same_flush = TexasHand(('Jh', '2c', '2h', '3h', '7h', '2s', '9h'))
        better_flush = TexasHand(('Jh', '2c', 'Ah', '3h', '7h', 'Ts', '9h'))
        straight = TexasHand(('Ah', '2s', '3d', '5c', '4c'))
        trips = TexasHand(('5d', '4c', '6d', '6h', '6c'))
        two_pair = TexasHand(('6d', '5c', '5h', 'Ah', 'Ac'))
        better_two_pair = TexasHand(('Td', 'Th', 'Ad', 'Ac', '6h'))
        pair = TexasHand(('Ah', '2d', '2s', '3c', '5c'))
        ace_pair = TexasHand(('Ac', 'As', '2s', '3d', '6c'))
        better_kicker = TexasHand(('Ac', 'As', 'Ts', '3d', '6c'))
        high_card = TexasHand(('Kh', 'Ah', 'Qh', '2h', '3s'))
        other_high_card = TexasHand(('Ks', 'As', 'Qs', '2h', '3s'))

        # Test random hand type comparisons
        self.assertTrue(royal_flush > straight_flush)
        self.assertTrue(royal_flush > trips)
        self.assertTrue(straight_flush > full_house)
        self.assertTrue(trips > two_pair)
        self.assertTrue(high_card < pair)
        self.assertTrue(straight <= flush)

        # Test rank levels within hands
        self.assertTrue(better_two_pair > two_pair)
        self.assertTrue(better_flush > flush)
        self.assertTrue(better_kicker > ace_pair)

        # Test for ties
        self.assertEqual(better_two_pair, better_two_pair)
        self.assertEqual(same_full_house, full_house)
        self.assertEqual(other_high_card, high_card)
        self.assertEqual(same_flush, flush)


class TestFastTexasHands(unittest.TestCase):

    def test_comparisons(self):
        table = HandTable()
        royal_flush = table[('Jd', 'As', 'Js', 'Ks', 'Qs', 'Ts', '2c')]
        straight_flush = table[('7d', '2c', '8d', 'Jd', '9d', '3d', 'Td')]
        four = table[('2h', '2c', '3d', '5c', '7d', '2d', '2s')]
        full_house = table[('As', 'Jd', 'Qs', 'Jc', '2c', 'Ac', 'Ah')]
        same_full_house = table[('As', 'Js', '2s', 'Jc', '2c', 'Ac', 'Ah')]
        flush = table[('Jh', '2c', '2h', '3h', '7h', 'As', '9h')]
        same_flush = table[('Jh', '2c', '2h', '3h', '7h', '2s', '9h')]
        better_flush = table[('Jh', '2c', 'Ah', '3h', '7h', 'Ts', '9h')]
        straight = table[('Ah', '2s', '3d', '5c', '4c')]
        trips = table[('5d', '4c', '6d', '6h', '6c')]
        two_pair = table[('6d', '5c', '5h', 'Ah', 'Ac')]
        better_two_pair = table[('Td', 'Th', 'Ad', 'Ac', '6h')]
        pair = table[('Ah', '2d', '2s', '3c', '5c')]
        ace_pair = table[('Ac', 'As', '2s', '3d', '6c')]
        better_kicker = table[('Ac', 'As', 'Ts', '3d', '6c')]
        high_card = table[('Kh', 'Ah', 'Qh', '2h', '3s')]
        other_high_card = table[('Ks', 'As', 'Qs', '2h', '3s')]

        # Test random hand type comparisons
        self.assertTrue(royal_flush > straight_flush)
        self.assertTrue(royal_flush > trips)
        self.assertTrue(straight_flush > full_house)
        self.assertTrue(trips > two_pair)
        self.assertTrue(high_card < pair)
        self.assertTrue(straight <= flush)

        # Test rank levels within hands
        self.assertTrue(better_two_pair > two_pair)
        self.assertTrue(better_flush > flush)
        self.assertTrue(better_kicker > ace_pair)

        # Test for ties
        self.assertEqual(better_two_pair, better_two_pair)
        self.assertEqual(same_full_house, full_house)
        self.assertEqual(other_high_card, high_card)
        self.assertEqual(same_flush, flush)


class TestCardAbstractions(unittest.TestCase):

    def test_preflop_abstractions(self):
        abst = PreflopAbstraction()
        self.assertEqual(len(abst.table), 1326)
        buckets = tuple(abst.table.values())
        n_buckets = len(np.unique(buckets))
        self.assertEqual(n_buckets, 169)
        self.assertEqual(abst[('Ac', 'Ad')], 336)

    def test_flop_abstraction(self):
        # Compare similar hands to see that they're in the same flop bucket
        # TODO: Possibly delete this test
        abstraction = FlopAbstraction()
        self.assertEqual(len(abstraction.abstraction.table), len(archetypal_flop_hands()))
        hand1 = ('Ac', 'Ad', '5d', '3s', '7c')
        hand2 = ('Ac', 'Ad', '5d', '3s', '8c')
        self.assertEqual(abstraction[hand1], abstraction[hand2])
        hand1 = ('Kh', '7h', '2h', '3h', '9h')
        hand2 = ('Kh', '8h', '2h', '3h', '9h')
        self.assertEqual(abstraction[hand1], abstraction[hand2])
        hand1 = ('9h', '7d', '5c', 'Qh', '9s')
        hand1 = ('9h', '7d', '5d', 'Qh', '9s')
        self.assertEqual(abstraction[hand1], abstraction[hand2])
        hand1 = ('Jd', 'Ad', '2c', '6c', '4d')
        hand2 = ('Jd', 'Ad', '2s', '6c', '4d')
        self.assertEqual(abstraction[hand1], abstraction[hand2])
        hand1 = ('3c', 'Qd', '5d', '7s', '4s')
        hand1 = ('3c', 'Qd', '7s', '6s', '4s')
        self.assertEqual(abstraction[hand1], abstraction[hand2])
        hand1 = ('Ts', '3c', '5c', 'Jd', '7d')
        hand2 = ('Ts', 'Tc', '5c', 'Jd', '7d')
        self.assertNotEqual(abstraction[hand1], abstraction[hand2])
        hand1 = ('9h', '8s', 'Js', 'Kh', 'Qs')
        hand2 = ('9h', '7s', 'Js', 'Kh', 'Qs')
        hand3 = ('9h', '8s', 'As', 'Kh', 'Qs')
        self.assertEqual(abstraction[hand1], abstraction[hand2])
        self.assertNotEqual(abstraction[hand1], abstraction[hand3])
        hand1 = ['Td', '4h', 'Th', 'Qs', '4s']
        hand2 = ['Td', '4h', 'Th', 'Jc', '4s']
        hand3 = ['Td', 'Th', 'Th', 'Qs', '4s']
        self.assertEqual(abstraction[hand1], abstraction[hand2])
        self.assertNotEqual(abstraction[hand1], abstraction[hand3])

    def test_archetypes(self):
        hand = ('6h', '8c', 'Td', 'Jd', 'Ah')
        truth = ('6s', '8h', 'As', 'Jd', 'Td')
        self.assertEqual(archetypal_hand(hand), truth)
        hand = ('2d', '7c', '2c', 'As', 'Ah')
        truth = ('2s', '7h', '2h', 'Ac', 'Ad')
        self.assertEqual(archetypal_hand(hand), truth)
        hand = ('Jc', 'Ad', 'Ts', '2c', 'Js')
        truth = ('As', 'Jh', '2h', 'Jd', 'Td')
        self.assertEqual(archetypal_hand(hand), truth)
        hand = ('5s', '5d', '2h', '6d', '6c')
        truth = ('5h', '5s', '2d', '6c', '6s')
        self.assertEqual(archetypal_hand(hand), truth)
        hand = ('7c', 'Ah', '9c', 'Ts', '5h')
        truth = ('7s', 'Ah', '5h', '9s', 'Td')
        self.assertEqual(archetypal_hand(hand), truth)
        hand = ('As', '2d', '4d', 'Ad', '7c')
        truth = ('2s', 'Ah', '4s', '7d', 'As')
        self.assertEqual(archetypal_hand(hand), truth)
        hand1 = ('5s', '5h', '2c', '6d', '6c')
        hand2 = ('5s', '5h', '2d', '6c', '6d')
        self.assertEqual(archetypal_hand(hand1), archetypal_hand(hand2))

    def test_hands(self):
        hands = pickle.load(open(ARCHETYPAL_TURN_FILENAME, 'rb'))
        deck = get_deck()
        for i in range(1000):
            hand = np.random.choice(deck, 6, replace=False)
            hand = archetypal_hand(hand)
            self.assertTrue(hand in hands)

    def test_equity(self):
        hand = ('Ac', 'Ad', 'As', 'Ah', '2h', '9c', 'Tc')
        self.assertTrue(equity(hand, samples=1000) > 0.9)
        hand = ('4c', '7d', 'As', '2h', '9h', '9c', 'Tc')
        self.assertTrue(equity(hand, samples=1000) < 0.2)

class ClusterTests(unittest.TestCase):

    def test_flop_coefficient(self):
        # This test makes sure that the average distance within clusters is less than the average
        # distance between the cluster and other hands.
        equities = pickle.load(open('flop_equity.pkl', 'rb'))
        abstraction = FlopAbstraction()
        hands = list(equities.keys())
        np.random.shuffle(hands)
        n_buckets = np.max(list(abstraction.abstraction.table.values())) + 1
        samples = 50
        for i in range(n_buckets):
            cluster = []
            for hand in hands:
                if abstraction[hand] == i:
                    cluster.append(hand)
                    if len(cluster) > samples:
                        break
           # 1 Compute estimated average distance within cluster
            total_distance = 0
            counter = 0
            for hand1, hand2 in itertools.product(cluster[:samples], cluster[:samples]):
                total_distance += np.linalg.norm(equities[hand1] - equities[hand2])
                counter += 1
            inside_mean_distance = total_distance / counter
            # 2 Compute estimated average distance to random other hands
            total_distance = 0
            counter = 0
            for hand1, hand2 in itertools.product(cluster[:samples], hands[:samples]):
                total_distance += np.linalg.norm(equities[hand1] - equities[hand2])
                counter += 1
            outside_mean_distance = total_distance / counter
            # see that 1 < 2
            self.assertTrue(inside_mean_distance < outside_mean_distance)

    def test_turn_coefficient(self):
        # This test makes sure that the average distance within clusters is less than the average
        # distance between the cluster and other hands.
        equities = pickle.load(open('turn_equity.pkl', 'rb'))
        abstraction = TurnAbstraction()
        hands = list(equities.keys())
        np.random.shuffle(hands)
        n_buckets = np.max(list(abstraction.abstraction.table.values())) + 1
        samples = 50
        for i in range(n_buckets):
            cluster = []
            for hand in hands:
                if abstraction[hand] == i:
                    cluster.append(hand)
                    if len(cluster) > samples:
                        break
            # 1 Compute estimated average distance within cluster
            total_distance = 0
            counter = 0
            for hand1, hand2 in itertools.product(cluster[:samples], cluster[:samples]):
                total_distance += np.linalg.norm(equities[hand1] - equities[hand2])
                counter += 1
            inside_mean_distance = total_distance / counter
            # 2 Compute estimated average distance to random other hands
            total_distance = 0
            counter = 0
            for hand1, hand2 in itertools.product(cluster[:samples], hands[:samples]):
                total_distance += np.linalg.norm(equities[hand1] - equities[hand2])
                counter += 1
            outside_mean_distance = total_distance / counter
            # see that 1 < 2
            self.assertTrue(inside_mean_distance < outside_mean_distance)


class ActionTests(unittest.TestCase):

    def setUp(self):
        self.histories = (
            ActionHistory(preflop=('limp', 'call'),
                          flop=('half_pot', 'call'),
                          turn=('half_pot', 'call'),
                          river=('half_pot', 'call')),
            ActionHistory(preflop=('raise', '3-bet', 'call'),
                          flop=('all-in',)),
            ActionHistory(preflop=('raise',)),
            ActionHistory(preflop=('limp', 'raise', 'call'),
                          flop=('check', 'half_pot', 'min_raise', 'call'),
                          turn=('check',)),
            ActionHistory(preflop=('raise', '3-bet', '4-bet', 'call'),
                          flop=('half_pot', 'call'),
                          turn=('check', 'pot', 'min_raise', 'call'),
                          river=('check', 'check')),
            ActionHistory(preflop=('all-in',)),
            ActionHistory(preflop=('limp', 'raise', 'call'))
        )

    def test_pot_size(self):
        truth = 200 + 200 + 400 + 800
        self.assertEqual(self.histories[0].pot_size(), truth)
        self.assertEqual(self.histories[1].pot_size(), STACK_SIZE + 9*BIG_BLIND)
        self.assertEqual(self.histories[2].pot_size(), 3*BIG_BLIND)
        self.assertEqual(self.histories[3].pot_size(), 6*BIG_BLIND + 12*BIG_BLIND)
        self.assertEqual(self.histories[4].pot_size(), 600 * BIG_BLIND)
        self.assertEqual(self.histories[5].pot_size(), STACK_SIZE)
        self.assertEqual(self.histories[6].pot_size(), 6 * BIG_BLIND)


    def test_preflop_pot_size(self):
        history = ActionHistory(preflop=('limp', 'call'))
        # self.assertEqual(history.pot_size(), 2*BIG_BLIND)
        history = ActionHistory(preflop=('raise', 'call'))
        # self.assertEqual(history.pot_size(), 6*BIG_BLIND)
        history = ActionHistory(preflop=('limp', 'raise', '3-bet', '4-bet', 'all-in', 'call'))
        self.assertEqual(history.pot_size(), 2*STACK_SIZE)
        history = ActionHistory(preflop=('raise', '3-bet', 'call'))
        self.assertEqual(history.pot_size(), 18*BIG_BLIND)

    def test_street(self):
        self.assertEqual(self.histories[0].street(), 'over')
        self.assertEqual(self.histories[1].street(), 'flop')
        self.assertEqual(self.histories[2].street(), 'preflop')
        self.assertEqual(self.histories[6].street(), 'flop')

    def test_whose_turn(self):
        self.assertEqual(self.histories[0].whose_turn(), 0)
        self.assertEqual(self.histories[1].whose_turn(), 1)
        self.assertEqual(self.histories[2].whose_turn(), 1)
        self.assertEqual(self.histories[3].whose_turn(), 1)
        self.assertEqual(self.histories[4].whose_turn(), 0)
        self.assertEqual(self.histories[5].whose_turn(), 1)
        self.assertEqual(self.histories[6].whose_turn(), 0)

    def test_legal_actions(self):
        self.assertEqual(self.histories[0].legal_actions(), ())
        self.assertEqual(self.histories[1].legal_actions(), ('fold', 'call'))
        self.assertEqual(self.histories[2].legal_actions(), ('fold', 'call', '3-bet'))
        self.assertEqual(self.histories[3].legal_actions(), ('check', 'half_pot', 'pot', 'all-in'))
        self.assertEqual(self.histories[4].legal_actions(), ())
        self.assertEqual(self.histories[5].legal_actions(), ('fold', 'call'))
        self.assertEqual(self.histories[6].legal_actions(), ('check', 'half_pot', 'pot', 'all-in'))


    def test_hand_over(self):
        pass


class InfoSetTests(unittest.TestCase):

    def test_eq(self):
        pass

    def test_hash(self):
        pass






if __name__ == '__main__':
    unittest.main()
