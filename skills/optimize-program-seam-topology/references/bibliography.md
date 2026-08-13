# Research bibliography

This is a curated research index for the skill, not a claim that every method
belongs in every architecture analysis. Prefer the primary paper for factual
claims; reviews are marked as such. Links point to publisher, author, archive,
or institutional copies where practical.

## Program dependence, slicing, and restructuring

1. Jeanne Ferrante, Karl J. Ottenstein, and Joe D. Warren. “The Program
   Dependence Graph and Its Use in Optimization.” *ACM TOPLAS* 9(3), 1987.
   [DOI](https://doi.org/10.1145/24039.24041),
   [institutional record](https://digitalcommons.mtu.edu/michigantech-p/12533/).
   Foundational integration of control and data dependence.

2. Susan Horwitz, Thomas Reps, and David Binkley. “Interprocedural Slicing
   Using Dependence Graphs.” *ACM TOPLAS* 12(1), 1990.
   [Primary PDF](https://research.cs.wisc.edu/wpis/papers/toplas90.pdf),
   [DOI](https://doi.org/10.1145/77606.77608). Introduces the system dependence
   graph and context-sensitive interprocedural slicing.

3. Mark Weiser. “Program Slicing.” *IEEE Transactions on Software
   Engineering* SE-10(4), 1984.
   [DOI](https://doi.org/10.1109/TSE.1984.5010248). Establishes static slicing
   as behavior-preserving reduction relative to a slicing criterion.

4. Hiralal Agrawal and Joseph R. Horgan. “Dynamic Program Slicing.” *PLDI
   1990*.
   [ACM](https://doi.org/10.1145/93548.93576),
   [author-course PDF](https://www.cs.purdue.edu/homes/xyzhang/spring07/Papers/p246-agrawal.pdf).
   Defines workload-specific dynamic slices and dynamic dependence graphs.

5. Arun Lakhotia and Jean-Christophe Deprez. “Restructuring Programs by
   Tucking Statements into Functions.” *Information and Software Technology*
   40(11–12), 1998.
   [DOI](https://doi.org/10.1016/S0950-5849(98)00091-3),
   [PDF](https://citeseerx.ist.psu.edu/document?doi=afdc5a1ca48f35545b303f234da433c574dcb994&repid=rep1&type=pdf).
   The wedge/split/fold transformation connects slicing to function extraction.

## Modular design, architecture recovery, and change structure

6. David L. Parnas. “On the Criteria To Be Used in Decomposing Systems into
   Modules.” *Communications of the ACM* 15(12), 1972.
   [ACM open article](https://doi.org/10.1145/361598.361623). The classic
   information-hiding argument: decomposition criteria determine changeability
   and comprehensibility.

7. Kevin J. Sullivan, William G. Griswold, Yuanfang Cai, and Ben Hallen. “The
   Structure and Value of Modularity in Software Design.” Technical report,
   University of Virginia, 2001.
   [Institutional record/PDF](https://libraopen.lib.virginia.edu/public_view/q237hr920).
   Applies design-structure matrices and real-options reasoning to Parnas's KWIC
   decompositions.

8. Alan MacCormack, John Rusnak, and Carliss Baldwin. “Exploring the Structure
   of Complex Software Designs: An Empirical Study of Open Source and
   Proprietary Code.” *Management Science* 52(7), 2006.
   [DOI](https://doi.org/10.1287/mnsc.1060.0552). Uses design-structure
   matrices to compare and track software modularity.

9. Spiros Mancoridis, Brian S. Mitchell, Yih-Farn Chen, and Emden R. Gansner.
   “Bunch: A Clustering Tool for the Recovery and Maintenance of Software
   System Structures.” *ICSM 1999*.
   [Author copy](https://www.cs.drexel.edu/~mancors/papers/). Search-based
   clustering of module dependency graphs.

10. Brian S. Mitchell and Spiros Mancoridis. “On the Automatic Modularization
    of Software Systems Using the Bunch Tool.” *IEEE TSE* 32(3), 2006.
    [DOI](https://doi.org/10.1109/TSE.2006.31),
    [author PDF](https://architectingsoftware.github.io/pubs/TSE-0035-0304.pdf).
    Architecture recovery through graph partition fitness and search.

11. Kata Praditwong, Mark Harman, and Xin Yao. “Software Module Clustering as a
    Multi-Objective Search Problem.” *IEEE TSE* 37(2), 2011.
    [DOI](https://doi.org/10.1109/TSE.2010.26),
    [institutional record](https://scholars.ln.edu.hk/en/publications/software-module-clustering-as-a-multi-objective-search-problem/).
    Keeps cohesion and coupling objectives separate and studies Pareto search.

12. Mark Harman and Bryan F. Jones. “Search-Based Software Engineering.”
    *Information and Software Technology* 43(14), 2001.
    [DOI](https://doi.org/10.1016/S0950-5849(01)00189-6). Frames software
    engineering decisions as explicit search problems.

13. José Carrasco et al. “Decomposition of Monolith Applications Into
    Microservices Architectures: A Systematic Review.” *IEEE TSE* 49(8), 2023.
    [IEEE](https://ieeexplore.ieee.org/document/10160171). **Review.** Highlights
    the fragmented use of static, dynamic, and evolutionary evidence and weak
    benchmark standardization.

## Hypergraphs, multilevel partitioning, and network communities

14. George Karypis, Rajat Aggarwal, Vipin Kumar, and Shashi Shekhar.
    “Multilevel Hypergraph Partitioning: Applications in VLSI Domain.” *IEEE
    Transactions on VLSI Systems* 7(1), 1999.
    [DOI](https://doi.org/10.1109/92.748202),
    [PDF](https://limsk.ece.gatech.edu/book/papers/hmetis.pdf). Coarsen,
    partition, uncoarsen, and refine while representing nets as hyperedges.

15. George Karypis and Vipin Kumar. “Multilevel k-way Hypergraph Partitioning.”
    *DAC 1999*.
    [DOI](https://doi.org/10.1145/309847.309954). Direct multilevel k-way
    hypergraph methods.

16. Robert Paige and Robert E. Tarjan. “Three Partition Refinement Algorithms.”
    *SIAM Journal on Computing* 16(6), 1987.
    [DOI](https://doi.org/10.1137/0216062). Efficient relational coarsest
    partition computation; foundational for bisimulation minimization.

17. Martin Rosvall and Carl T. Bergstrom. “Maps of Random Walks on Complex
    Networks Reveal Community Structure.” *PNAS* 105(4), 2008.
    [Open article](https://pmc.ncbi.nlm.nih.gov/articles/PMC2234100/),
    [DOI](https://doi.org/10.1073/pnas.0706851105). The map equation finds
    modules by compressing dynamical flow.

18. Peter J. Mucha, Thomas Richardson, Kevin Macon, Mason A. Porter, and
    Jukka-Pekka Onnela. “Community Structure in Time-Dependent, Multiscale, and
    Multiplex Networks.” *Science* 328, 2010.
    [PubMed](https://pubmed.ncbi.nlm.nih.gov/20466926/),
    [PDF](https://people.maths.ox.ac.uk/porterm/papers/multislice.pdf),
    [DOI](https://doi.org/10.1126/science.1184819). Multislice networks couple
    node identities across time, relation types, and scales.

19. Tiago P. Peixoto. “Hierarchical Block Structures and High-Resolution Model
    Selection in Large Networks.” *Physical Review X* 4, 011047, 2014.
    [Open paper](https://journals.aps.org/prx/abstract/10.1103/PhysRevX.4.011047),
    [PDF](https://journals.aps.org/prx/pdf/10.1103/PhysRevX.4.011047). A nested
    generative block model with MDL-based hierarchy/model selection.

20. Maurice Queyranne. “Minimizing Symmetric Submodular Functions.”
    *Mathematical Programming* 82, 1998.
    [DOI](https://doi.org/10.1007/BF01585863),
    [bibliographic record](https://dblp.org/rec/journals/mp/Queyranne98.html).
    Gives a combinatorial `O(n^3)` oracle-call method for symmetric submodular
    minimization, relevant to certain cut objectives.

## Information theory and description length

21. Jorma Rissanen. “Modeling by Shortest Data Description.” *Automatica*
    14(5), 1978.
    [DOI](https://doi.org/10.1016/0005-1098(78)90005-5),
    [record](https://www.scienceopen.com/document?vid=1e833f6b-55ab-4df6-861f-3cf3905aa089).
    Foundational minimum-description-length model selection.

22. Naftali Tishby, Fernando C. Pereira, and William Bialek. “The Information
    Bottleneck Method.” *Allerton Conference*, 1999.
    [arXiv](https://arxiv.org/abs/physics/0004057). Compresses one variable while
    retaining information relevant to another.

23. Claude E. Shannon. “Coding Theorems for a Discrete Source With a Fidelity
    Criterion.” *IRE National Convention Record* 4, 1959. Foundational
    rate-distortion formulation. Use zero distortion for protected program
    observations.

24. Angsheng Li and Yicheng Pan. “Structural Information and Dynamical Complexity of Networks.”
    *IEEE Transactions on Information Theory* 62(6), 2016.
    [DOI](https://doi.org/10.1109/TIT.2016.2555904). Develops structural
    information/entropy measures for network partitions.

## Formal refinement, equivalence, and validation

25. David Park. “Concurrency and Automata on Infinite Sequences.” *5th GI
    Conference on Theoretical Computer Science*, 1981.
    [Institutional copy](https://wrap.warwick.ac.uk/id/eprint/47224/). Early
    simulation/bisimulation treatment for concurrent infinite behavior.

26. Martín Abadi and Leslie Lamport. “The Existence of Refinement Mappings.”
    *Theoretical Computer Science* 82(2), 1991.
    [Author PDF](https://lamport.azurewebsites.net/pubs/abadi-existence.pdf),
    [DOI](https://doi.org/10.1016/0304-3975(91)90224-P). Establishes conditions
    and auxiliary-variable techniques for refinement mappings.

27. Leslie Lamport. “The Temporal Logic of Actions.” *ACM TOPLAS* 16(3), 1994.
    [DOI](https://doi.org/10.1145/177492.177726). State/action temporal logic
    for safety and liveness specifications.

28. Amir Pnueli, Michael Siegel, and Eli Singerman. “Translation Validation.”
    *TACAS 1998*.
    [DOI/PDF](https://doi.org/10.1007/BFb0054170). Validates each generated
    translation instead of proving the translator itself.

29. Xavier Leroy. “Formal Verification of a Realistic Compiler.”
    *Communications of the ACM* 52(7), 2009.
    [CompCert publication page](https://compcert.org/publi.html),
    [PDF](https://www.cs.cmu.edu/~15811/papers/compcert.pdf),
    [DOI](https://doi.org/10.1145/1538788.1538814). Mechanized semantic
    preservation for a realistic compiler.

30. Nuno P. Lopes, Juneyoung Lee, Chung-Kil Hur, Zhengyang Liu, and John Regehr.
    “Alive2: Bounded Translation Validation for LLVM.” *PLDI 2021*.
    [DOI](https://doi.org/10.1145/3453483.3454030),
    [project](https://github.com/AliveToolkit/alive2). Practical bounded
    relational checking of compiler transformations.

31. John C. Reynolds. “Separation Logic: A Logic for Shared Mutable Data
    Structures.” *LICS 2002*.
    [DOI](https://doi.org/10.1109/LICS.2002.1029817). Formal reasoning about
    disjoint mutable ownership and local effects.

32. David G. Clarke, John M. Potter, and James Noble. “Ownership Types for
    Flexible Alias Protection.” *OOPSLA 1998*.
    [DOI](https://doi.org/10.1145/286936.286947). Ownership structure as a
    control on aliasing and representation exposure.

## Compositional and geometric lenses

33. Brendan Fong and David I. Spivak. *An Invitation to Applied Category
    Theory: Seven Sketches in Compositionality*. 2019.
    [arXiv](https://arxiv.org/abs/1803.05316),
    [author PDF](https://dspivak.net/7Sketches.pdf). Wiring diagrams,
    hypergraph categories, open systems, and local-to-global behavior.

34. David I. Spivak. “The Operad of Wiring Diagrams: Formalizing a Graphical
    Language for Databases, Recursion, and Plug-and-Play Circuits.” 2013.
    [arXiv](https://arxiv.org/abs/1305.0297). Hierarchical composition through
    typed wiring diagrams.

35. Jakob Hansen and Robert Ghrist. “Toward a Spectral Theory of Cellular
    Sheaves.” *Journal of Applied and Computational Topology* 3, 2019.
    [DOI](https://doi.org/10.1007/s41468-019-00038-7). Extends spectral graph
    methods to local data plus compatibility/restriction maps.

36. Gabriel Peyré, Marco Cuturi, and Justin Solomon. “Gromov-Wasserstein
    Averaging of Kernel and Distance Matrices.” *ICML 2016*.
    [PMLR](https://proceedings.mlr.press/v48/peyre16.html),
    [PDF](https://proceedings.mlr.press/v48/peyre16.pdf). Relational alignment
    when domains have different sizes and no row correspondence.

37. Tadao Murata. “Petri Nets: Properties, Analysis and Applications.”
    *Proceedings of the IEEE* 77(4), 1989.
    [DOI](https://doi.org/10.1109/5.24143). **Review/tutorial.** Formal models
    for causality, concurrency, conflict, and synchronization.

## Protein folding: foundations and landscapes

38. Christian B. Anfinsen. “Principles That Govern the Folding of Protein
    Chains.” *Science* 181, 1973.
    [PubMed](https://pubmed.ncbi.nlm.nih.gov/4124164/),
    [DOI](https://doi.org/10.1126/science.181.4096.223). Thermodynamic
    hypothesis and sequence/native-state relation under suitable conditions.

39. Cyrus Levinthal. “How to Fold Graciously.” In *Mössbauer Spectroscopy in
    Biological Systems*, 1969.
    [Archival PDF](https://faculty.cc.gatech.edu/~turk/bio_sim/articles/proteins_levinthal_1969.pdf).
    The combinatorial-search argument later called Levinthal's paradox.

40. Joseph D. Bryngelson and Peter G. Wolynes. “Spin Glasses and the
    Statistical Mechanics of Protein Folding.” *PNAS* 84(21), 1987.
    [PubMed/PMC](https://pubmed.ncbi.nlm.nih.gov/3478708/),
    [DOI](https://doi.org/10.1073/pnas.84.21.7524). Introduces energy-landscape
    and frustration ideas to folding.

41. Joseph D. Bryngelson, José N. Onuchic, Nicholas D. Socci, and Peter G.
    Wolynes. “Funnels, Pathways, and the Energy Landscape of Protein Folding: A
    Synthesis.” *Proteins* 21(3), 1995.
    [arXiv](https://arxiv.org/abs/chem-ph/9411008),
    [DOI](https://doi.org/10.1002/prot.340210302). Statistical landscape,
    multiple routes, metastability, and kinetic traps.

42. José N. Onuchic, Peter G. Wolynes, Zaida Luthey-Schulten, and Nicholas D.
    Socci. “Toward an Outline of the Topography of a Realistic Protein-Folding
    Funnel.” *PNAS* 92(8), 1995.
    [DOI](https://doi.org/10.1073/pnas.92.8.3626). Quantitative funnel
    topography, barriers, smoothness, and ruggedness.

43. Ken A. Dill and Hue Sun Chan. “From Levinthal to Pathways to Funnels.”
    *Nature Structural Biology* 4, 1997.
    [DOI](https://doi.org/10.1038/nsb0197-10),
    [PDF](https://biofold.org/pages/courses/pibn/docs/dill_nsb1997.pdf).
    **Perspective/review.** Explains ensemble, funnel, and multipath views.

44. José N. Onuchic, Zaida Luthey-Schulten, and Peter G. Wolynes. “Theory of
    Protein Folding: The Energy Landscape Perspective.” *Annual Review of
    Physical Chemistry* 48, 1997.
    [DOI](https://doi.org/10.1146/annurev.physchem.48.1.545),
    [PDF](https://frustraevo.bsc.es/static/references/frustra_ref1.pdf).
    **Review.** Minimal frustration and rugged, funneled landscapes.

## Protein contacts, domains, and topology

45. Darrell B. Wetlaufer. “Nucleation, Rapid Folding, and Globular Intrachain
    Regions in Proteins.” *PNAS* 70(3), 1973.
    [PubMed/PMC](https://pubmed.ncbi.nlm.nih.gov/4351801/),
    [DOI](https://doi.org/10.1073/pnas.70.3.697). Early argument for partly
    autonomous structural regions/domains.

46. Liisa Holm and Chris Sander. “Parser for Protein Folding Units.” *Proteins*
    19(3), 1994.
    [PubMed](https://pubmed.ncbi.nlm.nih.gov/7937738/),
    [DOI](https://doi.org/10.1002/prot.340190309). Contact-matrix eigenanalysis
    and bisection to identify folding units.

47. Kevin W. Plaxco, Kim T. Simons, and David Baker. “Contact Order,
    Transition State Placement and the Refolding Rates of Single Domain
    Proteins.” *Journal of Molecular Biology* 277(4), 1998.
    [PubMed](https://pubmed.ncbi.nlm.nih.gov/9545386/),
    [DOI](https://doi.org/10.1006/jmbi.1998.1645). Relates a topological contact
    statistic to folding kinetics in the studied protein class.

48. Faruck Morcos et al. “Direct-Coupling Analysis of Residue Coevolution
    Captures Native Contacts Across Many Protein Families.” *PNAS* 108(49),
    2011.
    [Open article](https://pmc.ncbi.nlm.nih.gov/articles/PMC3241805/),
    [DOI](https://doi.org/10.1073/pnas.1111471108). Uses a global maximum-entropy
    model to separate direct from indirect covariation.

## Protein frustration, function, and allostery

49. Diego U. Ferreiro, Joseph A. Hegler, Elizabeth A. Komives, and Peter G.
    Wolynes. “Localizing Frustration in Native Proteins.” *PNAS* 104(50), 2007.
    [Open article](https://pmc.ncbi.nlm.nih.gov/articles/PMC2148382/),
    [DOI](https://doi.org/10.1073/pnas.0709915104). Defines localized frustration
    and relates frustrated clusters to functional sites.

50. Pavel I. Zhuravlev and Garegin A. Papoian. “Protein Functional Landscapes,
    Dynamics, Allostery: A Tortuous Path Towards a Universal Theoretical
    Framework.” *Quarterly Reviews of Biophysics* 43, 2010. **Review.** Useful
    [PubMed](https://pubmed.ncbi.nlm.nih.gov/20819242/),
    [DOI](https://doi.org/10.1017/S0033583510000119). **Review.** Useful broader
    context; consult original allostery work for specific claims.

51. María Inés Freiberger et al. “Local Frustration around Enzyme Active
    Sites.” *PNAS* 116(10), 2019.
    [DOI](https://doi.org/10.1073/pnas.1819859116). Links local energetic
    frustration with functional catalytic regions.

52. Diego U. Ferreiro et al. “On the Role of Frustration in the Energy
    Landscapes of Allosteric Proteins.” *PNAS* 108(9), 2011.
    [Open article](https://pmc.ncbi.nlm.nih.gov/articles/PMC3048099/),
    [DOI](https://doi.org/10.1073/pnas.1018980108). Explores purposeful local
    violations of minimal frustration in conformational function.

## Cotranslational folding, chaperones, disorder, and evolution

53. Lisa D. Cabrita, Christopher M. Dobson, and John Christodoulou. “Protein
    Folding on the Ribosome.” *Current Opinion in Structural Biology* 20(1),
    2010. **Review.** Ribosome context and vectorial emergence.

54. Christopher A. Waudby, Christopher M. Dobson, and John Christodoulou.
    “Nature and Regulation of Protein Folding on the Ribosome.” *Trends in
    Biochemical Sciences* 44(11), 2019.
    [Open review](https://pmc.ncbi.nlm.nih.gov/articles/PMC7471843/),
    [DOI](https://doi.org/10.1016/j.tibs.2019.06.008). **Review.** Coupling
    between translation kinetics, nascent-chain landscapes, and chaperones.

55. Arthur L. Horwich, Wayne A. Fenton, Eli Chapman, and George W. Farr. “Two
    Families of Chaperonin: Physiology and Mechanism.” *Annual Review of Cell
    and Developmental Biology* 23, 2007.
    [DOI](https://doi.org/10.1146/annurev.cellbio.23.090506.123555). **Review.**
    Chaperonin mechanisms and protected folding environments.

56. Rohit V. Pappu, Xiaoling Wang, Alexandre Vitalis, and Stephanie L. Crick.
    “A Polymer Physics Perspective on Driving Forces and Mechanisms for Protein
    Aggregation.” *Archives of Biochemistry and Biophysics* 469, 2008.
    [DOI](https://doi.org/10.1016/j.abb.2007.08.033). **Review.** Relevant to
    aggregation/misfolding analogies, not a software design prescription.

57. Prakash Kulkarni et al. “Intrinsically Disordered Proteins: Ensembles at
    the Limits of Anfinsen's Dogma.” *Biophysics Reviews* 3(1), 2022.
    [Open review](https://pmc.ncbi.nlm.nih.gov/articles/PMC10903413/),
    [DOI](https://doi.org/10.1063/5.0080512). **Review.** Functional
    conformational ensembles and inducible structure.

58. M. E. J. Newman and R. Engelhardt. “Effects of Selective Neutrality on the
    Evolution of Molecular Species.” *Proceedings of the Royal Society B* 265,
    1998.
    [Open article](https://pmc.ncbi.nlm.nih.gov/articles/PMC1689204/),
    [DOI](https://doi.org/10.1098/rspb.1998.0438). Neutral networks and
    genotype-phenotype landscape structure.

59. Claus O. Wilke. “Adaptive Evolution on Neutral Networks.” *Bulletin of
    Mathematical Biology* 63(4), 2001.
    [PubMed](https://pubmed.ncbi.nlm.nih.gov/11497165/),
    [arXiv](https://arxiv.org/abs/physics/0101021),
    [DOI](https://doi.org/10.1006/bulm.2001.0244). Dynamics on equal-fitness
    genotype networks and transitions between robust regions.

60. Sam F. Greenbury, Sebastian E. Ahnert, and Ard A. Louis. “The Structure of
    Genotype-Phenotype Maps Makes Fitness Landscapes Navigable.” *Nature
    Ecology & Evolution* 6, 2022.
    [Accepted manuscript](https://www.repository.cam.ac.uk/items/2f70aa27-7c2a-4130-a304-27d36a078aef).
    High-dimensional neutral networks can make apparently rugged landscapes
    navigable.

## Reading order for a new application

1. Parnas for the design criterion.
2. Ferrante plus Horwitz/Reps/Binkley for semantic dependency.
3. Rissanen plus Peixoto for granularity/model selection.
4. Karypis et al. for multilevel hypergraph search.
5. Abadi/Lamport or translation validation for preservation.
6. Anfinsen, Bryngelson/Onuchic/Wolynes, Ferreiro, and the cotranslational work
   for the biological lens.
7. The domain-specific papers only when their assumptions match the decision.
